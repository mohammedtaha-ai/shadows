// The frontend code map, checked against the tree. `docs/frontend/codebase/README.md`
// is written by hand; this test refuses the three ways it can be wrong: a source
// file with no row, a row naming a path that does not exist, and a folder listed
// under Contracts without its contract.yaml. It also refuses a job stated with
// "and", which is how a file quietly takes a second responsibility.

/// <reference types="node" />
import { existsSync, readdirSync, readFileSync } from 'node:fs'
import path from 'node:path'
import { describe, expect, it } from 'vitest'

const webSrc = import.meta.dirname
const repoRoot = path.resolve(webSrc, '../..')
const mapPath = path.join(repoRoot, 'docs/frontend/codebase/README.md')

interface Row {
  file: string
  job: string
  reference: string
}

const mapText = readFileSync(mapPath, 'utf8').replace(/\r\n/g, '\n')

/** Rows of the ownership table: lines that start `| \`web/src/`. */
function parseRows(text: string): Row[] {
  return text
    .split('\n')
    .filter(line => line.startsWith('| `web/src/'))
    .map(line => {
      const [, file = '', job = '', reference = ''] = line.split('|').map(cell => cell.trim())
      return { file: file.replace(/`/g, ''), job, reference: reference.replace(/`/g, '') }
    })
}

/** Folders listed under `## Contracts`: bullets of the form `- \`web/src/…\``. */
function parseContracts(text: string): string[] {
  const section = text.split('## Contracts')[1]?.split('\n## ')[0] ?? ''
  return section
    .split('\n')
    .filter(line => line.startsWith('- `web/src/'))
    .map(line => line.replace(/^- `|`$/g, ''))
}

/** Every non-test `.ts`/`.tsx` under `web/src`, relative to the repository root. */
function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap(entry => {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) return sourceFiles(full)
    const isSource = /\.tsx?$/.test(entry.name) && !/\.test\.tsx?$/.test(entry.name)
    return isSource ? [path.relative(repoRoot, full).split(path.sep).join('/')] : []
  })
}

const rows = parseRows(mapText)

describe('frontend code map', () => {
  it('has a row for every non-test source file under web/src', () => {
    const mapped = new Set(rows.map(row => row.file))
    const missing = sourceFiles(webSrc).filter(file => !mapped.has(file)).sort()
    expect(
      missing,
      `no row in docs/frontend/codebase/README.md for: ${missing.join(', ')}. ` +
        'Add a row naming the one job each file holds.',
    ).toEqual([])
  })

  it('names only paths that exist on disk', () => {
    const absent = rows.map(row => row.file).filter(file => !existsSync(path.join(repoRoot, file)))
    expect(
      absent,
      `docs/frontend/codebase/README.md names paths that are not in the tree: ${absent.join(', ')}`,
    ).toEqual([])
  })

  it('gives every contract folder a contract.yaml, and states no job with "and"', () => {
    const problems: string[] = []
    for (const folder of parseContracts(mapText)) {
      if (!existsSync(path.join(repoRoot, folder, 'contract.yaml'))) {
        problems.push(`${folder}/contract.yaml is missing`)
      }
    }
    for (const row of rows) {
      if (/\band\b/i.test(row.job)) {
        problems.push(`${row.file} is described as "${row.job}", which states two jobs`)
      }
    }
    expect(problems, problems.join('\n')).toEqual([])
  })
})
