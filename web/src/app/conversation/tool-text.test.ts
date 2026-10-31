import { describe, expect, it } from 'vitest'
import { toolText } from './tool-text'

const TOOLS = [
  'workflow_list', 'workflow_get', 'task_get', 'draft_prepare', 'draft_start', 'plan_edit',
  'agreement_edit', 'agreement_get', 'agreement_list', 'agreement_review', 'agreement_start',
  'map_approve', 'map_propose', 'outline', 'plan_approve', 'roadmap_approve',
  'roadmap_propose', 'vision_approve', 'vision_propose', 'where_is', 'who_uses', 'workspace_get',
]

describe('toolText', () => {
  it('every Shadows MCP tool has a sentence', () => {
    for (const t of TOOLS) {
      const text = toolText(`mcp__shadows__${t}`)
      expect(text, t).toBeTruthy()
      expect(text, t).not.toContain('mcp__')
    }
    expect(toolText('mcp__shadows__plan_show')).toBeNull()
  })
  it('an unknown Shadows tool never shows its mcp__ prefix', () => {
    expect(toolText('mcp__shadows__brand_new')).toBe('Used Shadows: brand_new')
    expect(toolText('mcp__shadows__constructor')).toBe('Used Shadows: constructor')
  })
  it('another tool keeps its title', () => {
    expect(toolText('Read file')).toBe('Read file')
  })
})
