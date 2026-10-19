import React from 'react';
import './assistant.css';

// The one existing assistant. More bots need a backend management contract.
export function BotList() {
  return <div className="bot-list">
    <ul aria-label="Bots">
      <li><button type="button" className="bot-row" aria-pressed="true">
        <span className="bot-face" aria-hidden="true">✦</span>
        <span className="bot-copy"><strong>AYLUL</strong><small>Your work companion</small></span>
      </button></li>
    </ul>
    <button type="button" className="bot-create" disabled aria-describedby="bot-create-reason">Create bot</button>
    <small id="bot-create-reason" className="bot-reason">Not available yet: needs bot management from the account backend.</small>
  </div>;
}
