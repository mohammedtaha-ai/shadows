import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
export function draw3DShadowPaws(
  ctx: CanvasRenderingContext2D,
  px: number,
  py: number,
  bobY: number,
  state: Expression,
  time: number,
  perchingHands: boolean,
  glowIntensity: number,
): void {
  ctx.save();
  let leftX = -46 + px * 0.35;
  let leftY = 36 + py * 0.25 + Math.sin(time * 3.2) * 3.0;
  let rightX = 44 + px * 0.35;
  let rightY = 36 + py * 0.25 + Math.cos(time * 3.2) * 3.0;

  if (perchingHands) {
    leftX = -34;
    leftY = 43 - bobY * 0.7;
    rightX = 24;
    rightY = 43 - bobY * 0.7;
  } else if (state === "happy" || state === "success") {
    leftX = -56 + px * 0.3;
    leftY = 14 - Math.abs(Math.sin(time * 7)) * 8;
    rightX = 56 + px * 0.3;
    rightY = 14 - Math.abs(Math.cos(time * 7)) * 8;
  } else if (state === "working") {
    leftX = -52 + Math.cos(time * 6) * 5;
    leftY = 24 + Math.sin(time * 6) * 5;
    rightX = 52 - Math.cos(time * 6) * 5;
    rightY = 24 - Math.sin(time * 6) * 5;
  } else if (state === "thinking") {
    // Right paw raised near chin in contemplation!
    rightX = 22 + px * 0.6;
    rightY = 26 + py * 0.4 + Math.sin(time * 3) * 1.5;
  }

  [
    { x: leftX, y: leftY, rot: 0.18 },
    { x: rightX, y: rightY, rot: -0.18 },
  ].forEach((paw) => {
    ctx.save();
    ctx.translate(paw.x, paw.y);
    ctx.rotate(paw.rot);

    // 3D Spherical Paw Shader
    const pawGrad = ctx.createRadialGradient(-3, -3, 1, 0, 0, 12);
    pawGrad.addColorStop(0, "#4c356e");
    pawGrad.addColorStop(0.55, "#1d142b");
    pawGrad.addColorStop(1, "#09060e");

    ctx.fillStyle = pawGrad;
    ctx.strokeStyle = `rgba(192, 132, 252, ${0.55 * glowIntensity})`;
    ctx.lineWidth = 1.4;
    ctx.shadowColor = "#a855f7";
    ctx.shadowBlur = 8;

    ctx.beginPath();
    ctx.ellipse(0, 0, 9.5, 6.8, 0, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  });

  ctx.restore();
}
