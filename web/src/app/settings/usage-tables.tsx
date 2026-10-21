import type { UsageReport, ProviderUsage, ModelUsage } from '@/api/usage'

const title = (provider: ProviderUsage) => `${provider.provider_name}${provider.label ? ` (${provider.label})` : ''}`
const total = (row: ProviderUsage | ModelUsage) => row.tokens_in + row.tokens_out

function Counts({ row }: { row: ProviderUsage | ModelUsage }) {
  return <>
    <td>{row.requests}</td><td>{row.tokens_in}</td><td>{row.tokens_out}</td><td>{total(row)}</td>
    <td data-usage-cost>{row.cost == null || row.currency == null ? '–' : `${row.cost} ${row.currency}`}</td>
  </>
}
function CountHeaders() {
  return <>{['Requests', 'Input tokens', 'Output tokens', 'Total tokens', 'Cost'].map(label => <th scope="col" key={label}>{label}</th>)}</>
}

export function UsageTables({ report }: { report: UsageReport }) {
  const max = report.by_provider.reduce((value, row) => Math.max(value, total(row)), 1)
  const providers = new Map(report.by_provider.map(provider => [provider.provider_slug, title(provider)]))
  return <div className="space-y-6 text-sm [&_caption]:mb-3 [&_caption]:text-left [&_caption]:font-medium [&_th]:px-2 [&_th]:py-2 [&_th]:text-left [&_td]:px-2 [&_td]:py-2">
    <p className="text-xs text-muted-foreground">Tokens include only reported usage.</p>
    <div className="overflow-x-auto"><table className="w-full">
      <caption>By provider</caption><thead><tr><th scope="col">Provider</th><CountHeaders /><th scope="col">Tokens compared</th></tr></thead>
      <tbody>{report.by_provider.map(row => <tr key={row.provider_slug} className="border-t border-border">
        <th scope="row">{title(row)}</th><Counts row={row} />
        <td><div role="meter" aria-label={`Tokens for ${title(row)}`} aria-valuemin={0} aria-valuemax={max} aria-valuenow={total(row)} aria-valuetext={`${total(row)} tokens`} className="h-2 min-w-20 overflow-hidden rounded bg-secondary">
          <div className="h-full bg-accent-line" style={{ width: `${100 * total(row) / max}%` }} />
        </div></td>
      </tr>)}</tbody>
    </table></div>
    <div className="overflow-x-auto"><table className="w-full">
      <caption>By model</caption><thead><tr><th scope="col">Model</th><th scope="col">Provider</th><CountHeaders /></tr></thead>
      <tbody>{report.by_model.map(row => <tr key={JSON.stringify([row.provider_slug, row.model])} className="border-t border-border">
        <th scope="row">{row.model}</th><td>{providers.get(row.provider_slug) ?? row.provider_slug}</td><Counts row={row} />
      </tr>)}</tbody>
    </table></div>
  </div>
}
