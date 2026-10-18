import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
import type { SceneFrame } from "../types.ts";
import { renderCharacter } from "./character.ts";
export function draw3DHoloPedestal(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  lookX: number,
  lookY: number,
  time: number,
  state: Expression,
  glowIntensity: number,
  showPedestal: boolean,
): void {
  ctx.save();
  ctx.translate(x, y);

  // Soft ground shadow
  const floorShadow = ctx.createRadialGradient(0, 0, 6, 0, 0, 90);
  floorShadow.addColorStop(0, "rgba(5, 3, 10, 0.72)");
  floorShadow.addColorStop(1, "rgba(5, 3, 10, 0)");
  ctx.fillStyle = floorShadow;
  ctx.beginPath();
  ctx.ellipse(0, 0, 88, 22, 0, 0, Math.PI * 2);
  ctx.fill();

  if (!showPedestal) {
    ctx.restore();
    return;
  }

  // 3D Perspective Tilt based on vertical look
  const perspY = Math.max(0.16, 0.24 + lookY * 0.06);
  ctx.scale(1, perspY);

  const ringColor =
    state === "blocked"
      ? `rgba(244, 63, 94, ${0.38 * glowIntensity})`
      : state === "success"
        ? `rgba(251, 191, 36, ${0.42 * glowIntensity})`
        : `rgba(192, 132, 252, ${0.36 * glowIntensity})`;

  // Outer spinning rune ring
  ctx.save();
  ctx.rotate(time * 0.45 + lookX * 0.2);
  ctx.strokeStyle = ringColor;
  ctx.lineWidth = 2.2;
  ctx.setLineDash([14, 10]);
  ctx.beginPath();
  ctx.arc(0, 0, 74, 0, Math.PI * 2);
  ctx.stroke();
  ctx.setLineDash([]);
  ctx.restore();

  // Inner counter-rotating geometric ring
  ctx.save();
  ctx.rotate(-time * 0.7);
  ctx.strokeStyle = "rgba(232, 121, 249, 0.28)";
  ctx.lineWidth = 1.6;
  ctx.beginPath();
  for (let i = 0; i < 6; i++) {
    const a = (i / 6) * Math.PI * 2;
    const rx = Math.cos(a) * 48;
    const ry = Math.sin(a) * 48;
    if (i === 0) ctx.moveTo(rx, ry);
    else ctx.lineTo(rx, ry);
  }
  ctx.closePath();
  ctx.stroke();
  ctx.restore();

  ctx.restore();
}
export function renderScene(
  ctx: CanvasRenderingContext2D,
  frame: SceneFrame,
  settings: CompanionSettings,
): void {
  const f = frame.character;
  for (const p of frame.particles) {
    ctx.fillStyle = `rgba(216,180,254,${p.alpha * (0.35 + p.z * 0.4)})`;
    ctx.beginPath();
    ctx.arc(
      f.cx + p.x - f.lookX * 18 * p.z + Math.sin(f.time + p.phase) * 8,
      f.cy + p.y - f.lookY * 10 * p.z,
      p.radius * (0.6 + p.z * 0.6),
      0,
      Math.PI * 2,
    );
    ctx.fill();
  }
  if (!f.perchingHands)
    draw3DHoloPedestal(
      ctx,
      f.cx,
      f.cy + 95 * f.scale,
      f.lookX,
      f.lookY,
      f.time,
      f.expression,
      settings.glowIntensity,
      settings.showPedestal,
    );
  renderCharacter(ctx, f, settings);
}
