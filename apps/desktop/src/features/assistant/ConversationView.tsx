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

export function ConversationView({controller}: {controller: AssistantController}) {
  const {snapshot, draft, error, ready, busy, activeTurn, connection, signedIn} = controller;
  const tail = React.useRef<HTMLDivElement>(null);
  const progress = snapshot?.turns.map(t => t.id + t.state + t.output.length).join('|');
  React.useEffect(() => { tail.current?.scrollIntoView({block: 'nearest'}); }, [progress]);
  return <section className="assistant" aria-label="Assistant conversation">
    <div className="assistant-heading">
      <h2>Chat with AYLUL</h2>
      <button disabled={!signedIn || busy} onClick={() => void controller.create()}>New conversation</button>
    </div>
    {!signedIn ? <p>Sign in below to start a conversation.</p> : <>
      <p className="assistant-connection" role="status">
        {ready ? 'Connected' : 'The assistant is currently unavailable. Your conversation is saved.'}
        {connection === 'unavailable' && <button type="button" onClick={controller.reconnect}>Retry connection</button>}
      </p>
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
        <label htmlFor="assistant-message">Message</label>
        <textarea id="assistant-message" value={draft} disabled={busy || !!activeTurn} placeholder="Message AYLUL…" rows={3}
          onChange={e => controller.setDraft(e.target.value)}
          onKeyDown={e => {
            if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); void controller.send(); }
          }} />
        <div className="assistant-compose-footer">
          <span>Enter to send · Shift+Enter for a new line</span>
          {controller.uncertain && <button type="button" disabled={busy} onClick={() => void controller.retry()}>Retry send</button>}
          <button type="submit" disabled={!ready || busy || !!activeTurn || !draft.trim()}>Send message</button>
        </div>
      </form>
    </>}
    {error && <p role="alert">{error}</p>}
  </section>;
}
