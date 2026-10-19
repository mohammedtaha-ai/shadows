import React from 'react';
import type {AuthView} from '../../platform/auth';
import {useAssistantConversation} from './useAssistantConversation';
import {ConversationView} from './ConversationView';

// Standalone page: one hook owning its own state, for tests and single-surface use.
export function AssistantPage({auth}: {auth: AuthView | null}) {
  return <ConversationView controller={useAssistantConversation(auth)} />;
}
