import { useIsMutating, useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef } from 'react'
import { Ban, Check, Hand } from 'lucide-react'
import { agentToolsQuery, saveToolRulesAnswer, setToolRules, type ToolRule, type ToolRuleEntry } from '@/api/extensions'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

const choices = [
  { rule: 'allow', label: 'Allow', Icon: Check },
  { rule: 'ask', label: 'Ask before', Icon: Hand },
  { rule: 'deny', label: 'Block', Icon: Ban },
] as const

export function ToolRules({ slug, id }: { slug: string; id: string }) {
  const client = useQueryClient()
  const state = useQuery(agentToolsQuery(slug, id))
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const mutationKey = ['aylul-mcp-tool-rules', slug, id]
  const pending = useIsMutating({ mutationKey }) > 0
  const mutation = useMutation({ mutationKey, retry: false,
    mutationFn: (rules: ToolRuleEntry[]) => {
      attempt.current = attemptFor(attempt.current, { slug, id, rules })
      return setToolRules(slug, id, rules, attempt.current.commandId)
    },
    onSuccess: async saved => {
      attempt.current = null
      await saveToolRulesAnswer(client, slug, id, saved)
    },
    onSettled: () => { locked.current = false },
  })
  const save = (rules: ToolRuleEntry[]) => {
    if (locked.current || pending) return
    locked.current = true
    mutation.mutate(rules)
  }
  if (state.isPending) return <p role="status" className="text-xs text-muted-foreground">Loading tools…</p>
  if (state.error) return <div className="space-y-2"><ErrorLine error={state.error} /><Button type="button" variant="outline" disabled={state.isFetching} onClick={() => { void state.refetch() }}>Retry tools</Button></div>
  const { tools, rules, list_error: listError } = state.data
  const missing = rules.filter(rule => !tools.some(tool => tool.name === rule.tool))
  const setRule = (tool: string, rule: ToolRule) => save([...rules.filter(entry => entry.tool !== tool), { tool, rule }])
  return <div role="group" aria-label="Tool permissions" className="space-y-3 border-t border-border pt-3">
    {listError && <p role="alert" className="text-sm text-destructive">{listError}</p>}
    {mutation.error && <ErrorLine error={mutation.error} />}
    {pending && <p role="status" className="text-xs text-muted-foreground">Saving…</p>}
    {tools.length > 0 && <div role="group" aria-label="Set all tools" className="flex flex-wrap items-center gap-2">
      <span className="text-xs text-muted-foreground">Set all</span>
      {choices.map(({ rule, label }) => <Button key={rule} type="button" size="xs" variant="outline" disabled={pending} aria-label={`${label} all tools`} onClick={() => save([
        ...missing, ...tools.map(tool => ({ tool: tool.name, rule })),
      ])}>{rule === 'ask' ? 'Ask' : label}</Button>)}
    </div>}
    {tools.map(tool => {
      const current = rules.find(entry => entry.tool === tool.name)?.rule ?? 'ask'
      return <div key={tool.name} className="space-y-1">
        <div className="flex items-center justify-between gap-3">
          <span className="min-w-0 break-words text-sm font-medium">{tool.name}</span>
          <div role="group" aria-label={`Permission for ${tool.name}`} className="flex shrink-0 gap-1">
            {choices.map(({ rule, label, Icon }) => <Button key={rule} type="button" size="icon-sm" variant={current === rule ? 'secondary' : 'ghost'} disabled={pending} aria-pressed={current === rule} title={`${label} ${tool.name}`} onClick={() => setRule(tool.name, rule)}>
              <Icon aria-hidden="true" /><span className="sr-only">{label} {tool.name}</span>
            </Button>)}
          </div>
        </div>
        {tool.description && <p className="break-words text-xs text-muted-foreground">{tool.description}</p>}
      </div>
    })}
    {tools.length === 0 && !listError && <p className="text-xs text-muted-foreground">No tools offered by the server.</p>}
    {missing.length > 0 && <div className="space-y-2">
      <p className="text-xs text-muted-foreground">{listError ? 'Saved rules' : 'No longer offered by the server'}</p>
      {missing.map(entry => <div key={entry.tool} className="flex items-center justify-between gap-2 text-xs">
        <span className="min-w-0 break-words">{entry.tool} · {entry.rule}</span>
        <Button type="button" size="xs" variant="outline" disabled={pending} aria-label={`Clear ${entry.tool}`} onClick={() => save(rules.filter(rule => rule.tool !== entry.tool))}>Clear</Button>
      </div>)}
    </div>}
    {listError && <Button type="button" size="xs" variant="outline" disabled={pending || state.isFetching} onClick={() => { void state.refetch() }}>Retry tools</Button>}
  </div>
}
