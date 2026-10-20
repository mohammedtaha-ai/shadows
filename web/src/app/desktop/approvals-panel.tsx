import React from 'react';
import { aylulMessage, answerRequest, approveRule, dismissRefusal, previewRule,
  type AylulState, type Pending } from '../../desktop/activity-api';
import { RulePreviewGuard, type RulePreview } from './rule-preview';

export function ApprovalsPanel({ state, refresh }: { state: AylulState | null; refresh: () => Promise<void> }) {
  if (!state) return null;
  return <section aria-labelledby="requests"><h2 id="requests">Requests</h2>
    {state.pending.length === 0 ? <p>Nothing is waiting for you.</p> : <ul>
      {state.pending.map(p => <li key={p.id}>{p.kind === 'request'
        ? <RequestItem item={p} refresh={refresh} />
        : <RefusalItem item={p} refresh={refresh} />}</li>)}
    </ul>}
  </section>;
}

function RequestItem({ item, refresh }: { item: Pending; refresh: () => Promise<void> }) {
  const [note, setNote] = React.useState('');
  const answer = (behavior: 'allow' | 'deny') =>
    void answerRequest(item.id, behavior).then(refresh).catch(e => setNote(aylulMessage(String(e))));
  return <div>
    <ItemContext item={item} />
    <p>Claude Code asks to run <strong>{item.tool}</strong>: <code>{item.target}</code></p>
    <button onClick={() => answer('allow')}>Allow once</button>
    <button onClick={() => answer('deny')}>Deny</button>
    {note && <p role="alert">{note}</p>}
  </div>;
}

function RefusalItem({ item, refresh }: { item: Pending; refresh: () => Promise<void> }) {
  const [rule, setRule] = React.useState(item.proposedRule ?? '');
  const [plan, setPlan] = React.useState<RulePreview | null>(null);
  const [note, setNote] = React.useState('');
  const guard = React.useRef(new RulePreviewGuard()).current;
  const fail = (e: unknown) => {
    if (guard.shouldClear(e)) setPlan(null);
    setNote(aylulMessage(String(e)));
  };
  return <div>
    <ItemContext item={item} />
    <p>Auto mode refused <strong>{item.tool}</strong>: <code>{item.target}</code></p>
    {item.reason && <p>Reason: {item.reason}</p>}
    <label>Rule to allow it in this project
      <input value={rule} onChange={e => { guard.invalidate(); setRule(e.target.value); setPlan(null); }} /></label>
    {!item.proposedRule && <p>AYLUL could not propose a narrow rule. Write the exact command, file or site.</p>}
    {plan ? <>
      <p>AYLUL will change <code>{plan.path}</code> (a backup is kept):</p>
      <pre>{plan.lines.join('\n') || 'The rule is already there.'}</pre>
      <button onClick={() => {
        const approved = guard.approval(plan);
        void approveRule(item.id, approved.rule, approved.before).then(refresh).catch(fail);
      }}>Approve rule</button>
    </> : <button disabled={!rule.trim()} onClick={() => void guard.load(rule, value => previewRule(item.id, value))
      .then(next => { if (next) setPlan(next); }).catch(fail)}>Preview</button>}
    <button onClick={() => void dismissRefusal(item.id).then(refresh).catch(fail)}>Dismiss</button>
    {item.held ? <p>Claude is waiting; after approval it tries again by itself.</p>
      : <p>After approval, ask Claude to try again.</p>}
    {note && <p role="alert">{note}</p>}
  </div>;
}

function ItemContext({ item }: { item: Pending }) {
  return <p>Project: <code>{item.project || 'Unknown project'}</code> · Session: <code>{item.sessionId}</code></p>;
}
