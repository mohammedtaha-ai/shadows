import React from 'react';
import {assistantFor,type AssistantSelection,type AssistantSnapshot,type AssistantConversation} from '../../platform/assistant';
import type {AuthView} from '../../platform/auth';

export type Connection = 'connected' | 'unavailable';

export type AssistantController = {
  source?: 'local'|'remote';
  conversations: AssistantConversation[];
  selected: string;
  snapshot: AssistantSnapshot | null;
  draft: string;
  error: string;
  ready: boolean;
  busy: boolean;
  activeTurn: AssistantSnapshot['turns'][number] | undefined;
  pinned: ReadonlySet<string>;
  connection: Connection;
  uncertain: boolean;
  signedIn: boolean;
  setDraft: (text: string) => void;
  select: (id: string) => void;
  create: () => Promise<void>;
  send: () => Promise<void>;
  retry: () => Promise<void>;
  reconnect: () => void;
  stop: (turnId: string) => Promise<void>;
  recover: (turnId: string) => Promise<void>;
  togglePin: (id: string) => void;
};

type Intent = {id: string; text: string; conversation: string};

const POLL_MS = 1500;
const MAX_BYTES = 4096;
const waiting = (state: string) => ['submitting', 'running', 'stopping', 'unknown'].includes(state);

function message(code: unknown): string {
  const value = String(code);
  if (value.includes('verification_refused')) return 'Please sign in to continue.';
  if (value.includes('request_conflict')) return 'This conversation already has a pending request. Refresh before sending again.';
  if (value.includes('request_refused')) return 'The message could not be sent. Keep it under 4 KB.';
  return 'Cannot reach the assistant. Your conversation is saved; you can retry.';
}

function readPins(account: string): Set<string> {
  try {
    const raw = JSON.parse(localStorage.getItem('aylul-pinned:' + account) ?? '[]');
    return new Set(Array.isArray(raw) ? raw.filter((id): id is string => typeof id === 'string') : []);
  } catch {
    return new Set();
  }
}

export function useAssistantConversation(auth: AuthView | null, selection:AssistantSelection|null={source:'remote'}): AssistantController {
  const principal = auth?.state === 'signed_in' ? auth.principal : null;
  const local=selection?.source==='local'?selection.installationId:'';
  const account = local?'local:'+local:selection?.source==='remote'?principal?.accountId??'':'';
  const session = local||principal?.sessionId||'';
  const source=selection?.source??'remote';
  const assistant=React.useMemo(()=>assistantFor(selection??{source:'remote'},principal??undefined),[source,local,account,session]);
  const [conversations, setConversations] = React.useState<AssistantConversation[]>([]);
  const [selected, setSelected] = React.useState('');
  const [snapshot, setSnapshot] = React.useState<AssistantSnapshot | null>(null);
  const [draft, setDraft] = React.useState('');
  const [error, setError] = React.useState('');
  const [busy, setBusy] = React.useState(false);
  const [ready, setReady] = React.useState(false);
  const [connection, setConnection] = React.useState<Connection>('connected');
  const [uncertain, setUncertain] = React.useState(false);
  const [pinned, setPinned] = React.useState<ReadonlySet<string>>(new Set());
  const [listKey, setListKey] = React.useState(0);

  const sourceEpoch = React.useRef(0);
  const scope = source + ':' + account + ':' + session + ':' + selected + ':' + sourceEpoch.current;
  const live = React.useRef(scope);
  live.current = scope;
  const pending = React.useRef<Intent | null>(null);
  const sending = React.useRef(false);
  const pollFailed = React.useRef(false);
  const initial = React.useRef(crypto.randomUUID());
  const refreshNow = React.useRef<(() => void) | null>(null);
  const loaded = React.useRef(false);

  // Clear the old source before paint, including switches with colliding IDs.
  React.useLayoutEffect(() => {
    sourceEpoch.current++;
    setSnapshot(null);
    setConversations([]);
    setSelected('');
    setDraft('');
    setError('');
    setReady(false);
    setConnection('connected');
    setUncertain(false);
    setBusy(false);
    sending.current=false;
    setPinned(account ? readPins(account) : new Set());
    pending.current = null;
    loaded.current = false;
  }, [account, session,source]);

  React.useEffect(() => {
    if (!account) return;
    let active = true;
    void assistant.list().then(async list => {
      if (!active) return;
      loaded.current = true;
      setConnection('connected');
      setConversations(list.conversations);
      const saved = localStorage.getItem('aylul-conversation:' + account);
      if (list.conversations.length) {
        setSelected(list.conversations.find(c => c.id === saved)?.id ?? list.conversations[0]!.id);
      } else {
        const first = await assistant.create(initial.current);
        if (active) {
          setSelected(first.conversation.id);
          setSnapshot(first);
          setConversations([first.conversation]);
        }
      }
    }).catch(e => {
      if (active) {
        setConnection('unavailable');
        setError(message(e));
      }
    });
    return () => { active = false; };
  }, [account, session, listKey,assistant]);

  React.useEffect(() => {
    if (!account || !selected) return;
    localStorage.setItem('aylul-conversation:' + account, selected);
    const current = scope;
    let active = true;
    let refreshing = false;
    const refresh = async () => {
      if (refreshing) return;
      refreshing = true;
      try {
        const [status, data] = await Promise.all([assistant.status(), assistant.snapshot(selected)]);
        if (!active || live.current !== current) return;
        setReady(status.state === 'ready');
        setConnection('connected');
        setSnapshot(data);
        if (pollFailed.current) {
          pollFailed.current = false;
          setError('');
        }
        if (pending.current && data.turns.some(turn => turn.commandId === pending.current?.id)) {
          pending.current = null;
          setUncertain(false);
          setDraft('');
        }
      } catch (e) {
        if (active && live.current === current) {
          setReady(false);
          setConnection('unavailable');
          pollFailed.current = true;
          setError(message(e));
        }
      } finally {
        refreshing = false;
      }
    };
    refreshNow.current = () => void refresh();
    void refresh();
    const timer = setInterval(() => { if (!document.hidden) void refresh(); }, POLL_MS);
    return () => {
      active = false;
      refreshNow.current = null;
      clearInterval(timer);
    };
  }, [account, session, selected,assistant]);

  function select(id: string) {
    if (id === selected) return;
    pending.current = null;
    setUncertain(false);
    setDraft('');
    setSelected(id);
    setSnapshot(null);
    setError('');
  }

  async function create() {
    if (busy || !account) return;
    const current = scope;
    setBusy(true);
    pollFailed.current = false;
    setError('');
    try {
      const data = await assistant.create(crypto.randomUUID());
      if (live.current !== current) return;
      pending.current = null;
      setUncertain(false);
      setDraft('');
      setConversations(list => [data.conversation, ...list]);
      setSelected(data.conversation.id);
      setSnapshot(data);
    } catch (e) {
      if (live.current === current) setError(message(e));
    } finally {
      if(live.current===current)setBusy(false);
    }
  }

  async function dispatch(intent: Intent) {
    const current = scope;
    pending.current = intent;
    sending.current = true;
    setBusy(true);
    pollFailed.current = false;
    setError('');
    try {
      const data = await assistant.send(intent.conversation, intent.id, intent.text);
      if (live.current !== current) return;
      setSnapshot(data);
      setDraft('');
      setUncertain(false);
      pending.current = null;
    } catch (e) {
      if (live.current === current) {
        setUncertain(true);
        setError(message(e));
      }
    } finally {
      if(live.current===current){sending.current = false;setBusy(false);}
    }
  }

  async function send() {
    if (sending.current || !selected || !draft.trim() || !ready) return;
    if (new TextEncoder().encode(draft).length > MAX_BYTES) {
      setError('Keep your message under 4 KB.');
      return;
    }
    const kept = pending.current;
    const intent = kept?.conversation === selected && kept.text === draft
      ? kept
      : {id: crypto.randomUUID(), text: draft, conversation: selected};
    await dispatch(intent);
  }

  // Resends the uncertain send under its original command ID, never a new one.
  async function retry() {
    const intent = pending.current;
    if (sending.current || !intent || intent.conversation !== selected) return;
    await dispatch(intent);
  }

  function reconnect() {
    pollFailed.current = false;
    setError('');
    if (!loaded.current) setListKey(key => key + 1);
    else refreshNow.current?.();
  }

  async function control(turnId: string, recoverTurn: boolean) {
    const current = scope;
    pollFailed.current = false;
    setError('');
    setBusy(true);
    try {
      const data = await (recoverTurn ? assistant.recover(selected, turnId) : assistant.stop(selected, turnId));
      if (live.current === current) setSnapshot(data);
    } catch (e) {
      if (live.current === current) setError(message(e));
    } finally {
      if(live.current===current)setBusy(false);
    }
  }

  function togglePin(id: string) {
    if (!account) return;
    setPinned(old => {
      const next = new Set(old);
      if (!next.delete(id)) next.add(id);
      localStorage.setItem('aylul-pinned:' + account, JSON.stringify([...next]));
      return next;
    });
  }

  return {
    source,conversations, selected, snapshot, draft, error, ready, busy, pinned, connection, uncertain,
    activeTurn: snapshot?.turns.find(t => waiting(t.state)),
    signedIn: !!account,
    setDraft, select, create, send, retry, reconnect, togglePin,
    stop: turnId => control(turnId, false),
    recover: turnId => control(turnId, true),
  };
}
