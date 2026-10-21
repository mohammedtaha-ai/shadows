import { rgba, type Palette } from "./palette.ts";
import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
export function drawFourPointStar(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  radius: number,
  color: string,
): void {
  ctx.save();
  ctx.translate(x, y);
  ctx.fillStyle = color;
  ctx.shadowColor = color;
  ctx.shadowBlur = 12;
  ctx.beginPath();
  for (let i = 0; i < 4; i++) {
    const ang = (i * Math.PI) / 2;
    const nextAng = ang + Math.PI / 4;
    ctx.lineTo(Math.cos(ang) * radius, Math.sin(ang) * radius);
    ctx.lineTo(
      Math.cos(nextAng) * (radius * 0.28),
      Math.sin(nextAng) * (radius * 0.28),
    );
  }
  ctx.closePath();
  ctx.fill();
  ctx.restore();
}
export function drawStateEffects(
  ctx: CanvasRenderingContext2D,
  state: Expression,
  time: number,
  glowIntensity: number,
  sleepingMarks: readonly SleepingMark[],
  notificationPulse: number,
  palette: Palette,
): void {
  ctx.save();

  if (state === "listening") {
    ctx.strokeStyle = palette.sparkPale;
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    ctx.shadowColor = palette.sparkDeep;
    ctx.shadowBlur = 10;
    const pulse = Math.sin(time * 10) * 2;
    ctx.beginPath();
    ctx.moveTo(54 + pulse, -42 - pulse);
    ctx.lineTo(62 + pulse, -50 - pulse);
    ctx.moveTo(62 + pulse, -30 - pulse);
    ctx.lineTo(72 + pulse, -34 - pulse);
    ctx.stroke();
  }

  if (state === "speaking") {
    ctx.strokeStyle = palette.sparkSoft;
    ctx.lineWidth = 2.8;
    ctx.lineCap = "round";
    ctx.shadowColor = palette.spark;
    ctx.shadowBlur = 10;
    for (let i = 0; i < 3; i++) {
      const offset = Math.sin(time * 12 + i) * 2.5;
      ctx.beginPath();
      ctx.moveTo(52 + i * 7, -34 - i * 5 + offset);
      ctx.lineTo(59 + i * 9, -41 - i * 6 + offset);
      ctx.stroke();
    }
  }

  if (state === "thinking") {
    ctx.save();
    ctx.translate(64, -42 + Math.sin(time * 4) * 4);
    ctx.rotate(Math.sin(time * 2.5) * 0.12);
    ctx.font = 'bold 30px "Cairo", sans-serif';
    ctx.fillStyle = palette.glowMist;
    ctx.shadowColor = palette.glow;
    ctx.shadowBlur = 14;
    ctx.fillText("?", -8, 10);
    ctx.restore();
  }

  if (state === "working") {
    for (let r = 0; r < 3; r++) {
      ctx.save();
      ctx.translate(0, 6 + r * 15);
      ctx.rotate((r % 2 === 0 ? 1 : -1) * Math.sin(time * 2.5 + r) * 0.14);
      ctx.scale(1, 0.28);
      ctx.strokeStyle = r === 1 ? palette.sparkSoft : palette.glowSoft;
      ctx.lineWidth = r === 1 ? 3.2 : 2.2;
      ctx.shadowColor = palette.sparkDeep;
      ctx.shadowBlur = 14 * glowIntensity;
      ctx.beginPath();
      const startAng =
        (time * (3 + r) * (r % 2 === 0 ? 1 : -1)) % (Math.PI * 2);
      ctx.arc(0, 0, 82 + r * 11, startAng, startAng + Math.PI * 1.35);
      ctx.stroke();
      ctx.restore();
    }
  }

  if (state === "success") {
    const s1 = 10 + Math.sin(time * 6) * 3;
    const s2 = 13 + Math.cos(time * 6) * 3.5;
    drawFourPointStar(ctx, -68, -28 + Math.sin(time * 3) * 4, s1, "#fbbf24");
    drawFourPointStar(ctx, 68, -24 + Math.cos(time * 3) * 4, s2, "#fde047");
    drawFourPointStar(ctx, 54, -52 + Math.sin(time * 4) * 3, 6.5, "#fef08a");
  }

  if (state === "blocked") {
    ctx.save();
    ctx.translate(64, -42 + Math.sin(time * 6) * 3);
    ctx.shadowColor = "#f43f5e";
    ctx.shadowBlur = 16;
    ctx.fillStyle = "#f43f5e";
    ctx.strokeStyle = "#fda4af";
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(0, -15);
    ctx.lineTo(15, 12);
    ctx.lineTo(-15, 12);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    ctx.fillStyle = "#ffffff";
    ctx.font = "bold 16px Inter, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("!", 0, 9);
    ctx.restore();
  }

  if (state === "sleeping") {
    ctx.fillStyle = palette.sleep;
    ctx.shadowColor = palette.sleepGlow;
    ctx.shadowBlur = 10;
    sleepingMarks.forEach((z, idx) => {
      const prog = (time * 0.45 + z.offset) % 1;
      const zx = 48 + idx * 14 + Math.sin(prog * Math.PI * 2) * 5;
      const zy = -24 - prog * 46;
      ctx.globalAlpha = Math.sin(prog * Math.PI);
      ctx.font = `bold ${13 + idx * 4}px "Inter", sans-serif`;
      ctx.fillText("z", zx, zy);
    });
    ctx.globalAlpha = 1;
  }

  if (notificationPulse > 0.01) {
    ctx.save();
    const p = 1 - notificationPulse;
    ctx.strokeStyle = rgba(palette.spark, notificationPulse);
    ctx.lineWidth = 3;
    ctx.beginPath();
    ctx.arc(0, -10, 65 + p * 75, 0, Math.PI * 2);
    ctx.stroke();

    ctx.translate(62, -38);
    ctx.fillStyle = palette.glow;
    ctx.strokeStyle = palette.sparkPale;
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.arc(0, 0, 14, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.fillStyle = "#ffffff";
    ctx.font = "bold 13px Inter, sans-serif";
    ctx.textAlign = "center";
    ctx.fillText("🔔", 0, 5);
    ctx.restore();
  }

  ctx.restore();
}
