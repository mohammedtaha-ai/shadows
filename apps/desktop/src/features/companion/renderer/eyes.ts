import type { Palette } from "./palette.ts";
import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
export function drawExpressiveEye3D(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  side: number,
  scaleX: number,
  lookX: number,
  lookY: number,
  state: Expression,
  outfit: OutfitId,
  blinkProgress: number,
  time: number,
  glowIntensity: number,
  isMiniIcon: boolean,
  palette: Palette,
): void {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(scaleX, 1);

  const isBlocked = state === "blocked";
  const coreColor = isBlocked ? "#ffe4e6" : palette.core;
  const midGlow = isBlocked ? "#fb7185" : palette.spark;
  const outerBloom = isBlocked ? "#e11d48" : palette.bloom;

  ctx.shadowColor = midGlow;
  ctx.shadowBlur = (isMiniIcon ? 10 : 19) * glowIntensity;
  ctx.fillStyle = coreColor;
  ctx.strokeStyle = coreColor;
  ctx.lineCap = "round";

  // Happy & Success Crescent Arches (^ ^)
  if (state === "happy" || state === "success") {
    ctx.lineWidth = isMiniIcon ? 4.5 : 4.0;
    ctx.beginPath();
    ctx.arc(0, 4, 8.5, Math.PI * 1.15, Math.PI * 1.85, false);
    ctx.stroke();
    ctx.restore();
    return;
  }

  // Sleeping Closed Downward Slits
  if (state === "sleeping") {
    ctx.lineWidth = 3.0;
    ctx.globalAlpha = 0.85;
    ctx.beginPath();
    ctx.arc(0, 2, 7.5, Math.PI * 0.18, Math.PI * 0.82, false);
    ctx.stroke();
    ctx.restore();
    return;
  }

  // Standard Oval Eye Dimensions
  let rx = isMiniIcon ? 8.2 : 7.4;
  let ry = isMiniIcon ? 12.0 : 11.0;

  if (state === "listening") {
    rx *= 1.12;
    ry *= 1.18;
  } else if (state === "thinking") {
    ctx.translate(side * 2, -4);
    if (side === -1) ry *= 0.78;
  }

  ry *= Math.max(0.08, 1 - blinkProgress);

  // 3D Spherical Eye Gradient (Shifted toward look direction for 3D lens volume)
  const eyeGrad = ctx.createRadialGradient(
    lookX * 2.2,
    -ry * 0.22 + lookY * 2.0,
    1,
    0,
    0,
    Math.max(rx, ry) * 1.35,
  );
  eyeGrad.addColorStop(0, "#ffffff");
  eyeGrad.addColorStop(0.45, coreColor);
  eyeGrad.addColorStop(0.75, midGlow);
  eyeGrad.addColorStop(1, outerBloom);
  ctx.fillStyle = eyeGrad;

  ctx.beginPath();
  ctx.ellipse(0, 0, rx, ry, 0, 0, Math.PI * 2);
  ctx.fill();

  // 3D Cornea Glass Specular Highlight Dot (Gives 3D depth to the eye)
  if (!isMiniIcon && blinkProgress < 0.4) {
    ctx.shadowBlur = 0;
    ctx.fillStyle = "rgba(255, 255, 255, 0.92)";
    ctx.beginPath();
    ctx.ellipse(
      -2.0 + lookX * 1.5,
      -ry * 0.35 + lookY * 1.2,
      2.1,
      3.0,
      -0.2,
      0,
      Math.PI * 2,
    );
    ctx.fill();
  }

  // Determined / Annoyed Angled Brow Mask for Working, Blocked, Scarf, or Special
  const needsAngledBrow =
    state === "working" ||
    state === "blocked" ||
    (outfit === "scarf" && side === 1) ||
    outfit === "special";
  if (needsAngledBrow && blinkProgress < 0.5) {
    ctx.shadowBlur = 0;
    ctx.fillStyle = palette.brow;
    const browTilt = side === -1 ? 0.38 : -0.38;
    ctx.save();
    ctx.translate(0, -ry * 0.55);
    ctx.rotate(browTilt);
    ctx.fillRect(-rx * 1.4, -ry, rx * 2.8, ry * 1.15);
    ctx.restore();
  }

  ctx.restore();
}
