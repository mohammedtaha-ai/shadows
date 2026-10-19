import React from "react";
import type { Rect } from "../types.ts";
export function MockWindow({
  onPerch,
}: {
  onPerch: (rect: Rect) => void;
}): React.JSX.Element {
  const host = React.useRef<HTMLDivElement>(null);
  const drag = React.useRef<{ x: number; y: number } | null>(null);
  const [position, setPosition] = React.useState({ x: 30, y: 140 });
  const boundPosition = React.useCallback((x: number, y: number) => {
    const el = host.current,
      parent = el?.parentElement;
    if (!el || !parent) return { x, y };
    const maxX = Math.max(0, parent.clientWidth - el.offsetWidth);
    const maxY = Math.max(0, parent.clientHeight - el.offsetHeight);
    const minY = Math.min(100, maxY);
    return {
      x: Math.max(0, Math.min(maxX, x)),
      y: Math.max(minY, Math.min(maxY, y)),
    };
  }, []);
  const measure = React.useCallback(() => {
    const el = host.current;
    if (el) {
      const next = boundPosition(position.x, position.y);
      setPosition((old) => (old.x === next.x && old.y === next.y ? old : next));
      onPerch({
        x: next.x,
        y: next.y,
        width: el.offsetWidth,
        height: el.offsetHeight,
      });
    }
  }, [onPerch, position, boundPosition]);
  React.useEffect(() => {
    measure();
    const observer = new ResizeObserver(measure);
    if (host.current) observer.observe(host.current);
    if (host.current?.parentElement)
      observer.observe(host.current.parentElement);
    return () => observer.disconnect();
  }, [measure, position]);
  return (
    <div
      className="aylul-mock-window"
      ref={host}
      style={{ left: position.x, top: position.y }}
    >
      <button
        type="button"
        onPointerDown={(e) => {
          e.currentTarget.setPointerCapture(e.pointerId);
          drag.current = {
            x: e.clientX - position.x,
            y: e.clientY - position.y,
          };
        }}
        onPointerMove={(e) => {
          if (!drag.current || !host.current) return;
          setPosition(
            boundPosition(
              e.clientX - drag.current.x,
              e.clientY - drag.current.y,
            ),
          );
        }}
        onPointerUp={(e) => {
          drag.current = null;
          e.currentTarget.releasePointerCapture(e.pointerId);
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
      >
        Mock code window
      </button>
      <pre>Preview only{`\n`}No project execution.</pre>
    </div>
  );
}
