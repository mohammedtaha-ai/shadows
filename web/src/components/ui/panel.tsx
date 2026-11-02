import { cva, type VariantProps } from "class-variance-authority"
import type { ComponentProps } from "react"
import { cn } from "cn"

// `tone` picks one of the surfaces in index.css; the inset box is a quiet
// bordered area inside a panel, with no gradient or shadow of its own.
const panelVariants = cva("", {
  variants: {
    tone: {
      panel: "surface-panel",
      glow: "surface-glow",
      inset: "border border-border bg-muted/40",
    },
    size: {
      sm: "rounded-xl p-3",
      md: "rounded-2xl p-4",
      lg: "rounded-2xl p-6",
    },
  },
  defaultVariants: { tone: "panel", size: "md" },
})

function Panel({
  className,
  tone,
  size,
  ...props
}: ComponentProps<"div"> & VariantProps<typeof panelVariants>) {
  return <div data-slot="panel" className={cn(panelVariants({ tone, size }), className)} {...props} />
}

export { Panel, panelVariants }
