import React from 'react';
import { aylulMessage, installHooks, previewHooks, type AylulState, type Plan } from '../../platform/aylul';

const LABEL = { running: 'Working', waiting: 'Waiting for you', ended: 'Ended' } as const;
const projectName = (path: string) => path.split(/[\\/]/).filter(Boolean).at(-1) ?? path;

export function SessionsPanel({ state, refresh }: { state: AylulState | null; refresh: () => Promise<void> }) {
  const [plan, setPlan] = React.useState<Plan | null>(null);
  const [note, setNote] = React.useState('');
  const fail = (e: unknown) => setNote(aylulMessage(String(e)));
  if (!state) return <section aria-labelledby="sessions"><h2 id="sessions">Sessions</h2>
    <p role="status">{aylulMessage('sidecar_unavailable')}</p></section>;
  return <section aria-labelledby="sessions"><h2 id="sessions">Sessions</h2>
    {!state.hooksInstalled && <div>
      <p>Claude Code is not connected to AYLUL yet.</p>
      {plan ? <>
        <p>AYLUL will change <code>{plan.path}</code> (a backup is kept):</p>
        <pre>{plan.lines.join('\n')}</pre>
        <button onClick={() => void installHooks(plan.before)
          .then(r => { setPlan(null); setNote(r.backupPath ? `Backup: ${r.backupPath}` : ''); return refresh(); })
          .catch(fail)}>Connect</button>
        <button onClick={() => setPlan(null)}>Cancel</button>
      </> : <button onClick={() => void previewHooks().then(setPlan).catch(fail)}>Connect Claude Code</button>}
    </div>}
    {state.sessions.length === 0 ? <p>No sessions yet.</p> : <ul>
      {state.sessions.map(s => <li key={s.id}>
        <strong>{projectName(s.project)}</strong> · {LABEL[s.state]}
        {s.lastStep && <div>Last step: {s.lastStep.summary}</div>}
        {s.lastMessage && <div>Claude said: {s.lastMessage}</div>}
      </li>)}
    </ul>}
    {note && <p role="alert">{note}</p>}
  </section>;
}
