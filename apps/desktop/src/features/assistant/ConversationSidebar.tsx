import React from 'react';
import type {AssistantController} from './useAssistantConversation';
import './assistant.css';

export function ConversationSidebar({controller, query}: {controller: AssistantController; query: string}) {
  const {conversations, selected, pinned, signedIn} = controller;
  if (!signedIn) return <p className="workspace-note">Sign in to see your conversations.</p>;
  const needle = query.trim().toLowerCase();
  const matching = conversations.filter(c => !needle || c.title.toLowerCase().includes(needle));
  const rows = [...matching.filter(c => pinned.has(c.id)), ...matching.filter(c => !pinned.has(c.id))];
  if (!rows.length) return <p className="workspace-note">{needle ? 'No matching conversations.' : 'No conversations yet.'}</p>;
  return <ul className="conversation-list" aria-label="Conversations">
    {rows.map(c => <li key={c.id}>
      <button type="button" className="conversation-row" aria-pressed={c.id === selected} disabled={controller.busy}
        onClick={() => controller.select(c.id)}>
        <span className="conversation-title">{c.title}</span>
        <small>{new Date(c.createdAt).toLocaleString()}</small>
      </button>
      <button type="button" className="conversation-pin" aria-pressed={pinned.has(c.id)}
        aria-label={(pinned.has(c.id) ? 'Unpin ' : 'Pin ') + c.title} onClick={() => controller.togglePin(c.id)}>
        {pinned.has(c.id) ? 'Pinned' : 'Pin'}
      </button>
    </li>)}
  </ul>;
}
