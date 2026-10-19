import React from 'react';
import type {AssistantController} from './useAssistantConversation';
import './assistant.css';

// Read-only description of the assistant. Nothing here is saved or sent anywhere.
export function AssistantDetails({controller, onClose}: {controller: AssistantController; onClose: () => void}) {
  const state = !controller.signedIn ? 'Sign in to connect.'
    : controller.connection === 'unavailable' ? 'Unavailable. Your conversations are saved.'
    : controller.ready ? 'Connected.' : 'Reachable, but not ready to answer.';
  return <div className="assistant-details">
    <div className="details-title"><h2>Bot details</h2>
      <button type="button" aria-label="Close bot details" onClick={onClose}>Close</button></div>
    <section aria-label="Connection"><h3>Connection</h3><p>{state}</p></section>
    <section aria-label="Bot profile"><h3>Profile</h3>
      <label>Name<input value="AYLUL" readOnly disabled /></label>
      <label>Instructions<textarea readOnly disabled value="Managed by the account backend." /></label>
      <label>Model<input value="Managed by the account backend" readOnly disabled /></label>
      <p className="details-reason">Editing the name, instructions, provider or model is not available yet: it needs bot management from the account backend.</p>
      <button type="button" disabled aria-describedby="details-save-reason">Save changes</button>
      <small id="details-save-reason">Nothing can be saved yet.</small>
    </section>
    <section aria-label="Desktop character"><h3>Desktop character</h3>
      <p>The character is drawn only by its own desktop window. Show or hide it with the AYLUL controls in the top bar.</p></section>
  </div>;
}
