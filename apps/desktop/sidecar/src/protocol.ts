// What aylul-hook carries: one Claude Code hook event in, one reply out.
export type HookEvent = {
  session_id: string;
  hook_event_name: string;
  cwd?: string;
  transcript_path?: string;
  prompt_id?: string;
  permission_mode?: string;
  tool_name?: string;
  tool_input?: Record<string, unknown>;
  tool_response?: unknown;
  tool_use_id?: string;
  reason?: string;
  notification_type?: string;
  message?: string;
  last_assistant_message?: string;
  source?: string;
  /**
   * The session's project root (CLAUDE_PROJECT_DIR), added by the server from
   * aylul-hook's request; Claude Code does not send it. `cwd` follows `cd`, so
   * rules and sessions use this, never `cwd`.
   */
  project_dir?: string;
};

export type HookReply = { output?: Record<string, unknown> };
