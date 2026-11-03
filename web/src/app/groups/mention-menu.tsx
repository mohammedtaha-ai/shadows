export type Mentionable = { slug: string; name: string }

export function MentionMenu({ members, onPick }: {
  members: readonly Mentionable[]
  onPick: (slug: string) => void
}) {
  return (
    <ul aria-label="Mention an agent" className="max-h-40 overflow-y-auto rounded-lg border border-border bg-card p-1">
      {members.map(member => (
        <li key={member.slug}>
          <button type="button" className="w-full rounded px-2 py-1 text-start text-sm hover:bg-accent"
            aria-label={`@${member.slug}`} onClick={() => onPick(member.slug)}>
            <span dir="auto">{member.name}</span> <span className="text-muted-foreground">@{member.slug}</span>
          </button>
        </li>
      ))}
    </ul>
  )
}
