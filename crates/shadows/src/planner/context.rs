//! One job: reading a session's context breakdown on demand (spec §12.8).
//!
//! `/context` sent as a prompt answers the breakdown as one markdown message
//! and costs no model tokens, but it is not a Planner turn: it writes no
//! Operation and no entry, and its chunks never reach the conversation. It is
//! read only on an open, idle session that has answered a turn in this
//! adapter, within `SessionsConfig::context_wait`.

use super::Sessions;
use crate::agent::acp::AcpError;
use crate::agent::breakdown::{Category, parse};
use crate::agent::events::HarnessEvent;
use crate::thread::ThreadId;

/// Why there is no breakdown to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoBreakdown {
    NotOpen,
    NoTurnYet,
    Busy,
    TimedOut,
    Unreadable,
}

impl NoBreakdown {
    /// The reason in words a client shows as they are.
    pub fn reason(self) -> &'static str {
        match self {
            NoBreakdown::NotOpen => "the conversation's session is not open",
            NoBreakdown::NoTurnYet => {
                "no breakdown until the session's first answer: before it, reading one can take tens of seconds"
            }
            NoBreakdown::Busy => "a turn is running",
            NoBreakdown::TimedOut => "the harness did not answer in time",
            NoBreakdown::Unreadable => "the harness's answer could not be read",
        }
    }
}

impl Sessions {
    /// Sends `/context` on the thread's session and reads the breakdown from
    /// its answer. Takes the session's events while it runs, so a turn cannot
    /// interleave, and discards every one of them; gives them back after. On
    /// `TimedOut` the prompt is cancelled.
    pub async fn context(&self, thread: &ThreadId) -> Result<Vec<Category>, NoBreakdown> {
        let (opened, mut rx) = {
            let mut live = self.live.lock().await;
            let item = live.get_mut(thread).ok_or(NoBreakdown::NotOpen)?;
            if item.handle.has_exited() || item.opened.connection().is_closed() {
                return Err(NoBreakdown::NotOpen);
            }
            // A running turn says so first, even before the first answer.
            if item.events.is_none() {
                return Err(NoBreakdown::Busy);
            }
            if !item.answered {
                return Err(NoBreakdown::NoTurnYet);
            }
            let rx = item.events.take().ok_or(NoBreakdown::Busy)?;
            (item.opened.clone(), rx)
        };
        let (connection, session_id) = (opened.connection(), opened.session_id.as_str());
        // Notifications from before belong to nothing this reads.
        while rx.try_recv().is_ok() {}
        let mut text = String::new();
        let prompt = connection.prompt(session_id, "/context", &[]);
        tokio::pin!(prompt);
        let answer = tokio::time::timeout(self.config.context_wait, async {
            loop {
                tokio::select! {
                    answer = &mut prompt => break answer,
                    event = rx.recv() => match event {
                        Some(HarnessEvent::Chunk { text: chunk, .. }) => text.push_str(&chunk),
                        Some(_) => {}
                        None => break Err(AcpError::Closed),
                    },
                }
            }
        })
        .await;
        if answer.is_err() {
            // The events go back only once the cancelled `/context` has
            // answered (or `cancel_wait` passed), so its late chunks cannot
            // land in the next turn's conversation.
            connection.cancel(session_id);
            let _ = tokio::time::timeout(self.config.cancel_wait, &mut prompt).await;
        } else {
            while let Ok(event) = rx.try_recv() {
                if let HarnessEvent::Chunk { text: chunk, .. } = event {
                    text.push_str(&chunk);
                }
            }
        }
        self.give_back_events(thread, &opened, rx).await;
        match answer {
            Err(_) => Err(NoBreakdown::TimedOut),
            Ok(Err(_)) => Err(NoBreakdown::Unreadable),
            Ok(Ok(_)) => parse(&text).ok_or(NoBreakdown::Unreadable),
        }
    }
}
