import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
import { drawTrue3DCrystal } from "./crystals.ts";
import { drawFourPointStar } from "./effects.ts";
export function drawOutfitAccessory(
  ctx: CanvasRenderingContext2D,
  outfit: OutfitId,
  px: number,
  py: number,
  leftBrimX: number,
  rightBrimX: number,
  brimY: number,
  topPeakX: number,
  topPeakY: number,
  time: number,
  glowIntensity: number,
  isMiniIcon: boolean,
): void {
  if (outfit === "default") return;

  ctx.save();

  // 1. CROWN: Royal golden 3D crown with central 3D violet gem
  if (outfit === "crown") {
    ctx.save();
    ctx.translate(px * 0.35, -48 + py * 0.35);
    ctx.rotate(px * 0.004);

    // 3D Back inner rim of crown
    ctx.fillStyle = "#78350f";
    ctx.beginPath();
    ctx.ellipse(0, 4, 20, 4.5, 0, 0, Math.PI * 2);
    ctx.fill();

    const crownGrad = ctx.createLinearGradient(-22, -18, 22, 6);
    crownGrad.addColorStop(0, "#fef08a");
    crownGrad.addColorStop(0.45, "#f59e0b");
    crownGrad.addColorStop(0.8, "#b45309");
    crownGrad.addColorStop(1, "#fde047");
    ctx.fillStyle = crownGrad;
    ctx.strokeStyle = "#fef9c3";
    ctx.lineWidth = 1.3;

    ctx.beginPath();
    ctx.moveTo(-20, 6);
    ctx.lineTo(-24, -12);
    ctx.lineTo(-11, -3);
    ctx.lineTo(0, -19);
    ctx.lineTo(11, -3);
    ctx.lineTo(24, -12);
    ctx.lineTo(20, 6);
    ctx.quadraticCurveTo(0, 10, -20, 6);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();

    // Central 3D diamond on crown
    drawTrue3DCrystal(
      ctx,
      0,
      -2,
      4.2,
      6.2,
      time * 1.5,
      0.1,
      glowIntensity,
      false,
    );
    ctx.restore();
  }

  // 2. MOON: Golden crescent moon brow crest + side crescents
  if (outfit === "moon") {
    ctx.save();
    ctx.translate(px * 0.45, -38 + py * 0.45);
    const moonGrad = ctx.createLinearGradient(-12, -10, 12, 10);
    moonGrad.addColorStop(0, "#fef08a");
    moonGrad.addColorStop(1, "#d97706");
    ctx.fillStyle = moonGrad;
    ctx.shadowColor = "#f59e0b";
    ctx.shadowBlur = 12;
    ctx.beginPath();
    ctx.arc(0, 0, 11, 0.15 * Math.PI, 0.85 * Math.PI, false);
    ctx.arc(0, -4, 9, 0.8 * Math.PI, 0.2 * Math.PI, true);
    ctx.closePath();
    ctx.fill();

    [-28, 28].forEach((sideX) => {
      ctx.beginPath();
      ctx.arc(sideX, 5, 5, 0, Math.PI * 2);
      ctx.fill();
    });
    ctx.restore();
  }

  // 3. SCARF: Cozy 3D wrapped scarf around neck base
  if (outfit === "scarf") {
    ctx.save();
    ctx.translate(px * 0.2, 30 + py * 0.25);
    const scarfGrad = ctx.createLinearGradient(-52, -10, 52, 20);
    scarfGrad.addColorStop(0, "#5b21b6");
    scarfGrad.addColorStop(0.5, "#2e1065");
    scarfGrad.addColorStop(1, "#4c1d95");
    ctx.fillStyle = scarfGrad;
    ctx.strokeStyle = "#d97706";
    ctx.lineWidth = 2;

    ctx.beginPath();
    ctx.roundRect(-48, -6, 96, 20, 10);
    ctx.fill();
    ctx.stroke();

    ctx.beginPath();
    ctx.roundRect(-42, 4, 84, 15, 8);
    ctx.fill();
    ctx.stroke();

    ctx.beginPath();
    ctx.moveTo(-28, 12);
    ctx.lineTo(-36, 36 + Math.sin(time * 3) * 2);
    ctx.lineTo(-16, 34 + Math.sin(time * 3) * 2);
    ctx.lineTo(-14, 12);
    ctx.closePath();
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  // 4. STARRY: Golden star pins & draped celestial chains on hood
  if (outfit === "starry") {
    ctx.save();
    ctx.strokeStyle = "#fbbf24";
    ctx.lineWidth = 1.5;
    ctx.setLineDash([2, 2]);
    ctx.beginPath();
    ctx.moveTo(18 + px * 0.3, -38);
    ctx.quadraticCurveTo(38 + px * 0.3, -10, 58 + px * 0.2, 18);
    ctx.stroke();
    ctx.setLineDash([]);

    drawFourPointStar(ctx, -26 + px * 0.3, -34 + py * 0.3, 6.5, "#fde047");
    drawFourPointStar(ctx, 28 + px * 0.3, -24 + py * 0.3, 5.5, "#fbbf24");
    drawFourPointStar(ctx, 48 + px * 0.2, 6 + py * 0.2, 4.8, "#fde047");
    ctx.restore();
  }

  // 5. WINTER: Cute fluffy 3D blue-and-white winter pom-pom beanie
  if (outfit === "winter") {
    ctx.save();
    ctx.translate(px * 0.25, -42 + py * 0.3);
    const hatGrad = ctx.createRadialGradient(-12, -26, 4, 0, -10, 52);
    hatGrad.addColorStop(0, "#7dd3fc");
    hatGrad.addColorStop(0.5, "#0284c7");
    hatGrad.addColorStop(1, "#0c4a6e");
    ctx.fillStyle = hatGrad;
    ctx.beginPath();
    ctx.arc(0, 6, 48, Math.PI, 0, false);
    ctx.closePath();
    ctx.fill();

    const cuffGrad = ctx.createLinearGradient(0, -6, 0, 14);
    cuffGrad.addColorStop(0, "#ffffff");
    cuffGrad.addColorStop(1, "#cbd5e1");
    ctx.fillStyle = cuffGrad;
    ctx.strokeStyle = "#94a3b8";
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.roundRect(-52, -4, 104, 18, 9);
    ctx.fill();
    ctx.stroke();

    const pomBounce = Math.sin(time * 4) * 2.5;
    ctx.beginPath();
    ctx.arc(-46, -14 + pomBounce, 12.5, 0, Math.PI * 2);
    ctx.fill();
    ctx.stroke();
    ctx.restore();
  }

  // 6. HEADPHONES: Sleek 3D dark-violet glowing over-ear headphones
  if (outfit === "headphones") {
    ctx.save();
    ctx.strokeStyle = "#3b0764";
    ctx.lineWidth = 7.5;
    ctx.beginPath();
    ctx.arc(px * 0.2, -6 + py * 0.2, 64, Math.PI * 1.08, Math.PI * 1.92, false);
    ctx.stroke();

    ctx.strokeStyle = "#c084fc";
    ctx.lineWidth = 2.2;
    ctx.beginPath();
    ctx.arc(px * 0.2, -6 + py * 0.2, 64, Math.PI * 1.12, Math.PI * 1.88, false);
    ctx.stroke();

    [-62 + px * 0.2, 62 + px * 0.2].forEach((cupX, idx) => {
      ctx.save();
      ctx.translate(cupX, 4 + py * 0.25);
      ctx.rotate(idx === 0 ? 0.15 : -0.15);

      // 3D Outer cup thickness
      ctx.fillStyle = "#0f0720";
      ctx.beginPath();
      ctx.ellipse(idx === 0 ? -3 : 3, 0, 10, 19, 0, 0, Math.PI * 2);
      ctx.fill();

      ctx.fillStyle = "#1e1b4b";
      ctx.strokeStyle = "#c084fc";
      ctx.lineWidth = 2.2;
      ctx.shadowColor = "#d946ef";
      ctx.shadowBlur = 12 * glowIntensity;
      ctx.beginPath();
      ctx.ellipse(0, 0, 10, 19, 0, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();

      ctx.strokeStyle = "#f0abfc";
      ctx.lineWidth = 1.6;
      ctx.beginPath();
      ctx.ellipse(0, 0, 5, 11, 0, 0, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    });
    ctx.restore();
  }

  // 7. HALO: Large radiant 3D neon-violet celestial halo ring above hood
  if (outfit === "halo") {
    ctx.save();
    ctx.translate(px * 0.25, -64 + py * 0.25 + Math.sin(time * 3) * 2.5);
    ctx.rotate(-0.08 + px * 0.003);
    ctx.strokeStyle = "#f0abfc";
    ctx.lineWidth = 4.8;
    ctx.shadowColor = "#d946ef";
    ctx.shadowBlur = 22 * glowIntensity;
    ctx.beginPath();
    ctx.ellipse(0, 0, 56, 13, 0, 0, Math.PI * 2);
    ctx.stroke();

    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 1.6;
    ctx.beginPath();
    ctx.ellipse(0, 0, 56, 13, 0, 0, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }

  // 8. SPECIAL: Arcane Arch-Mage Filigree & Extra Floating 3D Mini-Crystals
  if (outfit === "special") {
    ctx.save();
    ctx.strokeStyle = "#fbbf24";
    ctx.lineWidth = 2.6;
    ctx.shadowColor = "#f59e0b";
    ctx.shadowBlur = 10;
    ctx.beginPath();
    ctx.arc(px * 0.3, -44 + py * 0.3, 24, Math.PI * 0.2, Math.PI * 0.8, false);
    ctx.stroke();

    ctx.strokeStyle = "rgba(217, 70, 239, 0.55)";
    ctx.lineWidth = 2;
    [-1, 1].forEach((dir) => {
      ctx.beginPath();
      ctx.moveTo(dir * 55, 5);
      ctx.quadraticCurveTo(
        dir * (82 + Math.sin(time * 4) * 6),
        -15,
        dir * 72,
        25 + Math.cos(time * 3) * 6,
      );
      ctx.stroke();
    });

    drawTrue3DCrystal(
      ctx,
      -58,
      -28 + Math.sin(time * 4 + 1) * 4,
      5.2,
      9.0,
      -time * 2.0,
      0.2,
      glowIntensity,
      false,
    );
    drawTrue3DCrystal(
      ctx,
      60,
      -18 + Math.cos(time * 4) * 4,
      5.2,
      9.0,
      time * 2.0,
      -0.2,
      glowIntensity,
      false,
    );
    ctx.restore();
  }

  ctx.restore();
}
