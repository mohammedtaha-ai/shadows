use super::{
    binding::BindingView,
    client::{Client, Problem},
    projection::{Observation, PresenceInput, PresenceProjection, Projection, Scope},
};
use crate::core_connection::ReadyConnection;
use serde::Deserialize;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;

#[derive(Clone)]
pub struct Timing {
    pub snapshot: Duration,
    pub liveness: Duration,
    pub tick: Duration,
    pub retry: Duration,
    pub retry_cap: Duration,
    pub entry: Duration,
}
impl Default for Timing {
    fn default() -> Self {
        Self {
            snapshot: Duration::from_secs(10),
            liveness: Duration::from_secs(10),
            tick: Duration::from_millis(250),
            retry: Duration::from_millis(500),
            retry_cap: Duration::from_secs(10),
            entry: Duration::from_millis(250),
        }
    }
}
#[cfg(test)]
impl Timing {
    pub fn test() -> Self {
        Self {
            snapshot: Duration::from_secs(1),
            liveness: Duration::from_millis(80),
            tick: Duration::from_millis(10),
            retry: Duration::from_millis(10),
            retry_cap: Duration::from_millis(40),
            entry: Duration::from_millis(30),
        }
    }
}
type BindingGuard = Option<(u64, Option<String>)>;
pub struct Update {
    guard: BindingGuard,
    pub scope: Option<Scope>,
    pub projection: PresenceProjection,
    epoch: u64,
    current: Arc<AtomicU64>,
}
impl Update {
    pub fn matches_binding(&self, binding: &BindingView) -> bool {
        if let Some(scope) = self.scope.as_ref() {
            return binding.generation == scope.generation
                && binding.configuration_id.as_deref() == Some(scope.configuration_id.as_str());
        }
        self.guard
            .as_ref()
            .is_none_or(|(generation, configuration)| {
                binding.generation == *generation && binding.configuration_id == *configuration
            })
    }
    pub fn is_current(&self) -> bool {
        self.current.load(Ordering::SeqCst) == self.epoch
    }
}
type Notify = Arc<dyn Fn(Update) + Send + Sync>;
#[derive(Clone)]
struct Request {
    guard: BindingGuard,
    epoch: u64,
    binding: Option<(BindingView, ReadyConnection)>,
    stopped: PresenceProjection,
}
pub struct PresenceWatcher {
    current: Arc<AtomicU64>,
    requests: watch::Sender<Request>,
    last: Mutex<Option<(BindingView, ReadyConnection)>>,
}
impl PresenceWatcher {
    pub fn new(notify: Notify) -> Arc<Self> {
        Self::start(notify, Timing::default())
    }
    #[cfg(test)]
    pub fn with_timing(notify: Notify, timing: Timing) -> Arc<Self> {
        Self::start(notify, timing)
    }
    fn start(notify: Notify, timing: Timing) -> Arc<Self> {
        let current = Arc::new(AtomicU64::new(0));
        let (requests, mut input) = watch::channel(Request {
            guard: None,
            epoch: 0,
            binding: None,
            stopped: PresenceProjection::unbound(),
        });
        let generation = current.clone();
        // One manager owns one runner future. Replacing input drops old reads before opening the next stream.
        tauri::async_runtime::spawn(async move {
            loop {
                let request = input.borrow_and_update().clone();
                if let Some((binding, connection)) = request.binding {
                    tokio::select! {
                        biased;
                     result=input.changed()=>{if result.is_err(){break}},
                     _=run(binding,connection,request.epoch,generation.clone(),notify.clone(),timing.clone())=>{
                      if input.changed().await.is_err(){break}
                     }
                    }
                } else {
                    notify(Update {
                        guard: request.guard,
                        scope: None,
                        projection: request.stopped,
                        epoch: request.epoch,
                        current: generation.clone(),
                    });
                    if input.changed().await.is_err() {
                        break;
                    }
                }
            }
        });
        Arc::new(Self {
            current,
            requests,
            last: Mutex::new(None),
        })
    }
    pub fn replace(&self, binding: BindingView, connection: ReadyConnection) {
        let mut last = self.last.lock().unwrap();
        if last.as_ref() == Some(&(binding.clone(), connection.clone())) {
            return;
        }
        *last = Some((binding.clone(), connection.clone()));
        let epoch = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.requests.send_replace(Request {
            epoch,
            guard: None,
            binding: Some((binding, connection)),
            stopped: PresenceProjection::unbound(),
        });
    }
    pub fn sync(&self, binding: BindingView, ready: Option<ReadyConnection>) {
        let guard = Some((binding.generation, binding.configuration_id.clone()));
        if binding.agent.is_none() {
            self.stopped(PresenceProjection::unbound(), guard);
        } else if let Some(ready) = ready {
            self.replace(binding, ready);
        } else {
            self.stopped(
                PresenceProjection::unavailable("Core unavailable. Retry local Core in Settings."),
                guard,
            );
        }
    }
    pub fn stop(&self) {
        self.stopped(
            PresenceProjection::unavailable("Core unavailable. Retry local Core in Settings."),
            None,
        );
    }
    fn stopped(&self, projection: PresenceProjection, guard: BindingGuard) {
        let mut last = self.last.lock().unwrap();
        *last = None;
        let epoch = self.current.fetch_add(1, Ordering::SeqCst) + 1;
        self.requests.send_replace(Request {
            epoch,
            binding: None,
            stopped: projection,
            guard,
        });
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}
struct Session {
    scope: Scope,
    projection: Projection,
    epoch: u64,
    current: Arc<AtomicU64>,
    notify: Notify,
}
impl Session {
    fn apply(&mut self, observation: Observation) {
        let before = self.projection.value();
        let projection = self.projection.apply(
            PresenceInput {
                scope: self.scope.clone(),
                observation,
            },
            now(),
        );
        if projection != before {
            self.publish(projection)
        }
    }
    fn publish(&self, projection: PresenceProjection) {
        if self.current.load(Ordering::SeqCst) == self.epoch {
            (self.notify)(Update {
                guard: None,
                scope: Some(self.scope.clone()),
                projection,
                epoch: self.epoch,
                current: self.current.clone(),
            });
        }
    }
    async fn refresh(
        &mut self,
        client: &Client,
        thread: &str,
        cursor: i64,
        live_completed: Option<String>,
    ) -> Result<(), Problem> {
        let snapshot = client.snapshot(thread).await?;
        self.apply(Observation::Snapshot {
            cursor,
            operations: snapshot.operations,
            entries: snapshot.entries,
            live_completed,
        });
        Ok(())
    }
}
async fn run(
    binding: BindingView,
    connection: ReadyConnection,
    epoch: u64,
    current: Arc<AtomicU64>,
    notify: Notify,
    timing: Timing,
) {
    let scope = Scope {
        generation: binding.generation,
        configuration_id: connection.configuration_id.clone(),
        runtime_id: connection.runtime_id.clone(),
        thread_id: binding
            .target
            .as_ref()
            .map(|target| target.thread_id.clone()),
    };
    let projection = Projection::new(scope.clone(), binding.agent.is_some(), binding.previews);
    let mut state = Session {
        scope,
        projection,
        epoch,
        current,
        notify,
    };
    if binding.agent.is_none() {
        state.publish(PresenceProjection::unbound());
        return;
    }
    state.publish(state.projection.value());
    let client = match Client::new(connection, timing.snapshot) {
        Ok(client) => client,
        Err(problem) => {
            state.apply(Observation::Unavailable(problem.status()));
            return;
        }
    };
    let mut failures = 0u32;
    let mut cursor = 0;
    loop {
        let outcome = match client.validate(&binding).await {
            Ok(()) => {
                state.apply(Observation::Ready);
                if let Some(target) = binding.target.as_ref() {
                    session(
                        &client,
                        &binding,
                        &target.thread_id,
                        &mut state,
                        &mut cursor,
                        &mut failures,
                        &timing,
                    )
                    .await
                } else {
                    failures = 0;
                    state.apply(Observation::CaughtUp);
                    tokio::time::sleep(timing.liveness).await;
                    continue;
                }
            }
            Err(problem) => Err(problem),
        };
        let problem = outcome.err().unwrap_or(Problem::Transport);
        if problem.terminal() {
            state.apply(Observation::Unavailable(problem.status()));
            return;
        }
        if problem != Problem::Transport {
            state.apply(Observation::Unavailable(problem.status()));
            tokio::time::sleep(timing.liveness).await;
            continue;
        }
        state.apply(Observation::Disconnected);
        failures += 1;
        if failures >= 10 {
            state.apply(Observation::Unavailable(
                "Unavailable. Retry local Core in Settings.",
            ));
            return;
        }
        let delay = timing
            .retry
            .saturating_mul(1u32 << (failures - 1).min(10))
            .min(timing.retry_cap);
        tokio::time::sleep(delay).await;
    }
}
async fn session(
    client: &Client,
    binding: &BindingView,
    thread: &str,
    state: &mut Session,
    cursor: &mut i64,
    failures: &mut u32,
    timing: &Timing,
) -> Result<(), Problem> {
    let mut feed = client.subscribe(thread, *cursor).await?;
    // Subscribe before snapshot. Replay is suppressed until this stream's own caught-up.
    state.refresh(client, thread, *cursor, None).await?;
    let mut caught_up = false;
    let caught_up_deadline = tokio::time::sleep(timing.snapshot);
    tokio::pin!(caught_up_deadline);
    let mut ticks = tokio::time::interval(timing.tick);
    let mut liveness = tokio::time::interval(timing.liveness);
    liveness.tick().await;
    let mut entry_attempts = 0;
    let mut entry_at = 0;
    loop {
        tokio::select! {
         _=&mut caught_up_deadline,if !caught_up=>return Err(Problem::Transport),
         _=liveness.tick()=>{client.validate(binding).await?;state.apply(Observation::Ready);}
         _=ticks.tick()=>{
          state.apply(Observation::Tick);
          let clock=now();
          if state.projection.needs_entry(clock)&&entry_attempts<5&&clock>=entry_at{
           state.refresh(client,thread,*cursor,None).await?;
           entry_attempts+=1;entry_at=clock.saturating_add(timing.entry.saturating_mul(1u32<<entry_attempts).as_millis() as u64);
          }
          if !state.projection.needs_entry(clock){entry_attempts=0;entry_at=0;}
         }
         frame=feed.next()=>{
          let frame=frame.ok_or(Problem::Transport)??;
          match frame.event.as_str(){
           "caught-up"=>{
            #[derive(Deserialize)]struct CaughtUp{seq:i64}
            let value:CaughtUp=serde_json::from_str(&frame.data).map_err(|_|Problem::Invalid)?;
            if value.seq<*cursor{return Err(Problem::Invalid)}*cursor=value.seq;
            caught_up=true;*failures=0;state.apply(Observation::CaughtUp);
           }
           "durable"=>{
            #[derive(Deserialize)]struct Durable {seq:i64,kind:String,operation_id:Option<String>,thread_id:Option<String>}
            let value:Durable=serde_json::from_str(&frame.data).map_err(|_|Problem::Invalid)?;
            if value.seq<=0{return Err(Problem::Invalid)}
            if value.seq<=*cursor{continue}*cursor=value.seq;
            if value.thread_id.as_deref()!=Some(thread){continue}
            if value.kind.starts_with("Operation")||value.kind=="ThreadEntryAppended"{
             let completed=if caught_up&&value.kind=="OperationCompleted"{Some(value.operation_id.ok_or(Problem::Invalid)?)}else{None};
             state.refresh(client,thread,*cursor,completed).await?;
            }
           }
           "delta"|"subagent"=>{
            #[derive(Deserialize)]struct Progress {op:String,#[serde(default)]text:String}
            let value:Progress=serde_json::from_str(&frame.data).map_err(|_|Problem::Invalid)?;
            if !caught_up{continue}
            if state.projection.current_operation()!=Some(value.op.as_str()){state.refresh(client,thread,*cursor,None).await?;}
            if frame.event=="delta"{
             if !value.text.is_empty(){state.apply(Observation::Delta(value.op))}
            }else{state.apply(Observation::Tool(value.op))}
           }
           "turn-end"=>{if caught_up{state.refresh(client,thread,*cursor,None).await?;}}
           "lagged"|"fatal"=>return Err(Problem::Transport),
           _=>{}
          }
         }
        }
    }
}
