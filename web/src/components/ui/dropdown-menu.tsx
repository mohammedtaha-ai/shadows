// shadcn/ui's DropdownMenu on Base UI's Menu, in the base-nova style of the
// other components here. Written by hand from that registry's shape because
// the registry was out of reach when it was added; only the parts this app
// uses are here, and the rest are added the day a screen needs one.

import * as React from "react"
import { Menu as MenuPrimitive } from "@base-ui/react/menu"
import { CheckIcon } from "lucide-react"
import { cn } from "cn"

function DropdownMenu(props: MenuPrimitive.Root.Props) {
  return <MenuPrimitive.Root data-slot="dropdown-menu" {...props} />
}

function DropdownMenuTrigger(props: MenuPrimitive.Trigger.Props) {
  return <MenuPrimitive.Trigger data-slot="dropdown-menu-trigger" {...props} />
}

/** The popup fades and scales in from 96% next to its trigger;
 * `prefers-reduced-motion` keeps the change instant. */
function DropdownMenuContent({
  className,
  side = "bottom",
  align = "start",
  sideOffset = 4,
  ...props
}: MenuPrimitive.Popup.Props &
  Pick<MenuPrimitive.Positioner.Props, "side" | "align" | "sideOffset">) {
  return (
    <MenuPrimitive.Portal>
      <MenuPrimitive.Positioner
        className="isolate z-50 outline-none"
        side={side}
        align={align}
        sideOffset={sideOffset}
      >
        <MenuPrimitive.Popup
          data-slot="dropdown-menu-content"
          className={cn(
            "max-h-(--available-height) min-w-40 origin-(--transform-origin) overflow-x-hidden overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-lg outline-none transition-[opacity,scale] duration-100 ease-out data-ending-style:scale-96 data-ending-style:opacity-0 data-starting-style:scale-96 data-starting-style:opacity-0 motion-reduce:transition-none",
            className
          )}
          {...props}
        />
      </MenuPrimitive.Positioner>
    </MenuPrimitive.Portal>
  )
}

function DropdownMenuGroup(props: MenuPrimitive.Group.Props) {
  return <MenuPrimitive.Group data-slot="dropdown-menu-group" {...props} />
}

function DropdownMenuLabel({ className, ...props }: MenuPrimitive.GroupLabel.Props) {
  return (
    <MenuPrimitive.GroupLabel
      data-slot="dropdown-menu-label"
      className={cn("px-2 py-1.5 text-xs font-medium text-muted-foreground", className)}
      {...props}
    />
  )
}

const itemClass =
  "relative flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none data-disabled:pointer-events-none data-disabled:opacity-50 data-highlighted:bg-accent data-highlighted:text-accent-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4"

function DropdownMenuItem({ className, ...props }: MenuPrimitive.Item.Props) {
  return (
    <MenuPrimitive.Item data-slot="dropdown-menu-item" className={cn(itemClass, className)} {...props} />
  )
}

function DropdownMenuRadioGroup(props: MenuPrimitive.RadioGroup.Props) {
  return <MenuPrimitive.RadioGroup data-slot="dropdown-menu-radio-group" {...props} />
}

/** An item that shows a check while it is the group's value. Choosing one
 * closes the menu: every menu here picks one value, and Base UI keeps a radio
 * menu open by default, leaving it over the composer's Send button. */
function DropdownMenuRadioItem({
  className,
  children,
  ...props
}: MenuPrimitive.RadioItem.Props & { children?: React.ReactNode }) {
  return (
    <MenuPrimitive.RadioItem
      closeOnClick
      data-slot="dropdown-menu-radio-item"
      className={cn(itemClass, "pr-8", className)}
      {...props}
    >
      {children}
      <span className="pointer-events-none absolute right-2 flex size-3.5 items-center justify-center">
        <MenuPrimitive.RadioItemIndicator>
          <CheckIcon className="size-3.5" />
        </MenuPrimitive.RadioItemIndicator>
      </span>
    </MenuPrimitive.RadioItem>
  )
}

export {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
}
