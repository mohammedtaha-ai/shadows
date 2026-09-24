// shadcn/ui's Popover on Base UI's Popover, in the base-nova style of the
// other components here. Written by hand from that registry's shape because
// the registry was out of reach when it was added; only the parts this app
// uses are here.

import { Popover as PopoverPrimitive } from "@base-ui/react/popover"
import { cn } from "cn"

function Popover(props: PopoverPrimitive.Root.Props) {
  return <PopoverPrimitive.Root data-slot="popover" {...props} />
}

function PopoverTrigger(props: PopoverPrimitive.Trigger.Props) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />
}

/** The panel fades and scales in from 96% beside its trigger;
 * `prefers-reduced-motion` keeps the change instant. */
function PopoverContent({
  className,
  side = "bottom",
  align = "center",
  sideOffset = 6,
  ...props
}: PopoverPrimitive.Popup.Props &
  Pick<PopoverPrimitive.Positioner.Props, "side" | "align" | "sideOffset">) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Positioner
        className="isolate z-50"
        side={side}
        align={align}
        sideOffset={sideOffset}
      >
        <PopoverPrimitive.Popup
          data-slot="popover-content"
          className={cn(
            "w-72 origin-(--transform-origin) rounded-lg border border-border bg-popover p-3 text-sm text-popover-foreground shadow-lg outline-none transition-[opacity,scale] duration-100 ease-out data-ending-style:scale-96 data-ending-style:opacity-0 data-starting-style:scale-96 data-starting-style:opacity-0 motion-reduce:transition-none",
            className
          )}
          {...props}
        />
      </PopoverPrimitive.Positioner>
    </PopoverPrimitive.Portal>
  )
}

export { Popover, PopoverContent, PopoverTrigger }
