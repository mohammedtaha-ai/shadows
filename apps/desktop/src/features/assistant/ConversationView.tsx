import React from 'react';
import type {AssistantController} from './useAssistantConversation';
import './assistant.css';

const STATE_TEXT: Record<string, string> = {
  submitting: 'Sending…',
  running: 'Thinking…',
  unknown: 'The result is uncertain. Check the original request before sending again.',
  stopping: 'Stopping…',
  cancelled: 'Response stopped.',
  error: 'The response could not be completed.',
  done: 'No text was returned.',
};

type Props = {controller: AssistantController; inspectorOpen?: boolean; onInspector?: () => void};

const SendIcon = () => <svg aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="15" height="15" fill="none"
  stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m5 12 7-7 7 7" /><path d="M12 19V5" /></svg>;

export function ConversationView({controller, inspectorOpen, onInspector}: Props) {
  const {snapshot, draft, error, ready, busy, activeTurn, connection, signedIn} = controller;
  const tail = React.useRef<HTMLDivElement>(null);
  const progress = snapshot?.turns.map(t => t.id + t.state + t.output.length).join('|');
  React.useEffect(() => { tail.current?.scrollIntoView({block: 'nearest'}); }, [progress]);
  return <section className="assistant" aria-label="Assistant conversation">
    <header className="assistant-heading">
      <span className="assistant-face" aria-hidden="true">✦</span>
      <div className="assistant-who">
        <h2>AYLUL</h2>
        <small>Your work companion</small>
      </div>
      {signedIn && <span className="assistant-connection" role="status" data-state={connection === 'unavailable' ? 'unavailable' : ready ? 'ready' : 'waiting'}
        title={ready ? 'Connected' : 'The assistant is currently unavailable. Your conversation is saved.'}>
        <i aria-hidden="true" />{ready ? 'Connected' : 'Unavailable'}
        {connection === 'unavailable' && <button type="button" onClick={controller.reconnect}>Retry connection</button>}
      </span>}
      <button disabled={!signedIn || busy} onClick={() => void controller.create()}>New conversation</button>
      {onInspector && <button type="button" aria-expanded={!!inspectorOpen} onClick={onInspector}>Bot settings</button>}
    </header>
    {!signedIn ? <p>Sign in below to start a conversation.</p> : <>
      <div className="assistant-messages" role="log" aria-label="Messages" aria-live="polite">
        {!snapshot?.turns.length && <p className="assistant-empty">What would you like to talk about?</p>}
        {snapshot?.turns.map(turn => <React.Fragment key={turn.id}>
          <article className="assistant-message user"><strong>You</strong><p dir="auto">{turn.input}</p></article>
          <article className="assistant-message reply"><strong>AYLUL</strong>
            <p dir="auto">{turn.output || STATE_TEXT[turn.state]}</p>
            {turn.state === 'unknown' && <button disabled={busy} onClick={() => void controller.recover(turn.id)}>Recover original request</button>}
            {['running', 'submitting', 'stopping'].includes(turn.state) &&
              <button disabled={busy || turn.state === 'stopping'} onClick={() => void controller.stop(turn.id)}>Stop response</button>}
          </article>
        </React.Fragment>)}
        <div ref={tail} />
      </div>
      {snapshot?.truncated && <p>Older messages or long responses were shortened in this view.</p>}
      <form onSubmit={e => { e.preventDefault(); void controller.send(); }}>
        <textarea id="assistant-message" aria-label="Message" value={draft} disabled={busy || !!activeTurn} placeholder="Message AYLUL…" rows={3}
          onChange={e => controller.setDraft(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); void controller.send(); }
          }} />
        <div className="assistant-compose-footer">
          <span className="assistant-chip">Model: managed by the account backend</span>
          <span className="assistant-hint">Enter to send · Shift+Enter for a new line</span>
          {controller.uncertain && <button type="button" disabled={busy} onClick={() => void controller.retry()}>Retry send</button>}
          <button type="submit" className="assistant-send" aria-label="Send message" disabled={!ready || busy || !!activeTurn || !draft.trim()}><SendIcon /></button>
        </div>
      </form>
    </>}
    {error && <p role="alert">{error}</p>}
  </section>;
}
