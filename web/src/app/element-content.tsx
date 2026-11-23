// One job: one revision of a table or screen as text, its columns or its links.
import type { DesignElement } from '@/api/project-state'

export function ElementContent({ element, names }: {
  element: DesignElement
  /** An element id's name, when the caller knows it; the id is shown otherwise. */
  names?: (id: string) => string | undefined
}) {
  const name = (id: string) => names?.(id) ?? id
  const content = element.content
  return (
    <div className="space-y-2 text-xs">
      {element.deleted && <p className="text-destructive-foreground">Deleted in this revision</p>}
      <p dir="auto">{content.purpose}</p>
      {content.kind === 'table' ? (
        <table aria-label={`${content.name} columns`} className="w-full border-collapse text-left">
          <thead className="text-muted-foreground">
            <tr>
              <th className="pr-3 font-medium">Name</th>
              <th className="pr-3 font-medium">Type</th>
              <th className="pr-3 font-medium">Meaning</th>
              <th className="pr-3 font-medium">Key</th>
              <th className="font-medium">References</th>
            </tr>
          </thead>
          <tbody>
            {content.columns.map(column => (
              <tr key={column.name} className="align-top">
                <td dir="auto" className="pr-3 font-mono">{column.name}</td>
                <td className="pr-3 font-mono">{column.type_name}</td>
                <td dir="auto" className="pr-3">{column.meaning}</td>
                <td className="pr-3">{column.primary_key ? 'primary key' : ''}</td>
                <td dir="auto">
                  {column.references != null ? `→ ${name(column.references.table)}.${column.references.column}` : ''}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <>
          {content.route != null && <p dir="auto">Route: {content.route}</p>}
          {(content.leads_to ?? []).length > 0 && (
            <div>
              <p className="text-muted-foreground">Leads to</p>
              <ul aria-label="Leads to" className="list-disc pl-4">
                {(content.leads_to ?? []).map((link, index) => (
                  <li key={`${link.screen}:${index}`} dir="auto">{name(link.screen)} when {link.when}</li>
                ))}
              </ul>
            </div>
          )}
        </>
      )}
    </div>
  )
}
