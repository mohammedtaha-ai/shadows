import React from "react";
import type { CharacterFrame, CompanionSettings } from "../types.ts";
import { DIRECTIONS } from "../catalog.ts";
import { renderCharacter } from "../renderer/character.ts";
function Still({
  frame,
  settings,
  size,
  label,
}: {
  frame: CharacterFrame;
  settings: CompanionSettings;
  size: number;
  label: string;
}) {
  const canvas = React.useRef<HTMLCanvasElement>(null);
  React.useEffect(() => {
    const ctx = canvas.current?.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, size, size);
    renderCharacter(
      ctx,
      {
        ...frame,
        cx: size / 2,
        cy: size * 0.55,
        scale: size / 340,
        mini: size <= 64,
      },
      settings,
    );
  }, [frame, settings, size]);
  return (
    <figure>
      <canvas ref={canvas} width={size} height={size} aria-label={label} />
      <figcaption>{label}</figcaption>
    </figure>
  );
}
export function PreviewGrid({
  frame,
  settings,
}: {
  frame: CharacterFrame;
  settings: CompanionSettings;
}): React.JSX.Element {
  return (
    <div className="aylul-preview-grid" data-testid="preview-grid">
      {DIRECTIONS.map((d) => (
        <Still
          key={d.id}
          frame={{ ...frame, lookX: d.lx, lookY: d.ly, tilt: d.tilt }}
          settings={settings}
          size={90}
          label={d.id}
        />
      ))}
      {[256, 128, 64, 32, 16].map((size) => (
        <Still
          key={size}
          frame={frame}
          settings={settings}
          size={size}
          label={`${size}px`}
        />
      ))}
    </div>
  );
}
