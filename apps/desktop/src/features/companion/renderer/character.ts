import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
import { drawTrue3DCrystal } from "./crystals.ts";
import { drawExpressiveEye3D } from "./eyes.ts";
import { draw3DShadowPaws } from "./hands.ts";
import { drawOutfitAccessory } from "./outfits.ts";
import { drawStateEffects } from "./effects.ts";
export function renderCharacter(
  ctx: CanvasRenderingContext2D,
  frame: CharacterFrame,
  settings: CompanionSettings,
): void {
  const {
    cx,
    cy,
    scale,
    lookX,
    lookY,
    tilt,
    time,
    earringAngle,
    blinkProgress,
    outfit,
    perchingHands,
  } = frame;
  const state = frame.expression,
    isMiniIcon = frame.mini,
    showPaws = settings.showHands;
  const { depth3D, glowIntensity } = settings;

  ctx.save();
  ctx.translate(cx, cy);
  ctx.scale(scale, scale);

  // Breathing & state vertical bobbing
  let bobY = Math.sin(time * 2.6) * 4.5;
  let stateTilt = tilt;
  let squashX = 1.0;
  let squashY = 1.0;

  if (state === "happy" || state === "success") {
    bobY = Math.abs(Math.sin(time * 5.5)) * -7;
    squashX = 1 + Math.sin(time * 5.5) * 0.035;
    squashY = 1 - Math.sin(time * 5.5) * 0.035;
  } else if (state === "sleeping") {
    bobY = Math.sin(time * 1.3) * 2.5;
    squashX = 1 + Math.sin(time * 1.3) * 0.025;
    squashY = 1 - Math.sin(time * 1.3) * 0.025;
  } else if (state === "working") {
    bobY = Math.sin(time * 4.0) * 3.0;
  } else if (state === "blocked") {
    stateTilt += Math.sin(time * 22) * 0.025;
  }

  if (isMiniIcon) bobY *= 0.25;

  ctx.translate(0, bobY);
  ctx.rotate(stateTilt);
  ctx.scale(squashX, squashY);

  const isBlocked = state === "blocked";
  const isSuccess = state === "success";
  const px = lookX * 22 * (0.85 + depth3D * 0.15);
  const py = lookY * 14 * (0.85 + depth3D * 0.15);

  // =======================================================================
  // 1. 3D VOLUMETRIC AMBIENT BACKGLOW BEHIND CLOAK
  // =======================================================================
  if (!isMiniIcon) {
    const backGlow = ctx.createRadialGradient(-px * 0.2, -10, 8, 0, -10, 135);
    if (isBlocked) {
      backGlow.addColorStop(0, `rgba(244, 63, 94, ${0.32 * glowIntensity})`);
    } else if (isSuccess) {
      backGlow.addColorStop(0, `rgba(232, 121, 249, ${0.38 * glowIntensity})`);
      backGlow.addColorStop(
        0.55,
        `rgba(251, 191, 36, ${0.14 * glowIntensity})`,
      );
    } else {
      backGlow.addColorStop(0, `rgba(168, 85, 247, ${0.32 * glowIntensity})`);
    }
    backGlow.addColorStop(1, "rgba(168, 85, 247, 0)");
    ctx.fillStyle = backGlow;
    ctx.beginPath();
    ctx.arc(0, -10, 135, 0, Math.PI * 2);
    ctx.fill();
  }

  // =======================================================================
  // 2. WORKING STATE BACK ORBITAL RINGS (Behind cloak)
  // =======================================================================
  if (state === "working" && !isMiniIcon) {
    ctx.save();
    for (let r = 0; r < 2; r++) {
      ctx.save();
      ctx.translate(0, 10 + r * 18);
      ctx.rotate((r === 0 ? 1 : -1) * Math.sin(time * 2 + r) * 0.12);
      ctx.scale(1, 0.28);
      ctx.strokeStyle =
        r === 0 ? "rgba(232, 121, 249, 0.45)" : "rgba(168, 85, 247, 0.35)";
      ctx.lineWidth = 3.5;
      ctx.beginPath();
      ctx.arc(0, 0, 88 + r * 12, Math.PI, Math.PI * 2);
      ctx.stroke();
      ctx.restore();
    }
    ctx.restore();
  }

  // =======================================================================
  // 3. LOWER SHADOW SPIRIT BODY (3D Spherical Volume & Rim Bounce Light)
  // =======================================================================
  ctx.save();
  const bodyCenterX = px * 0.16;
  const bodyCenterY = 34 + Math.max(0, -py * 0.25);

  // 3D Spherical Body Gradient with top-left specular & bottom rim bounce
  const bodyGrad = ctx.createRadialGradient(
    bodyCenterX - 14 + px * 0.25,
    bodyCenterY - 6,
    4,
    bodyCenterX,
    bodyCenterY,
    52,
  );
  bodyGrad.addColorStop(0, "#2a1f3d"); // 3D Lit spherical highlight
  bodyGrad.addColorStop(0.45, "#150f21"); // Mid shadow volume
  bodyGrad.addColorStop(0.82, "#08050d"); // Deep core obsidian
  bodyGrad.addColorStop(1, "#1e1136"); // Subtle 3D rim bounce at edge

  ctx.fillStyle = bodyGrad;
  ctx.beginPath();
  ctx.ellipse(bodyCenterX, bodyCenterY, 44, 24, 0, 0, Math.PI * 2);
  ctx.fill();

  // 3D Ambient Occlusion Shadow cast by the Hood Brim onto the Body
  const aoGrad = ctx.createLinearGradient(
    0,
    bodyCenterY - 22,
    0,
    bodyCenterY + 6,
  );
  aoGrad.addColorStop(0, "rgba(3, 1, 7, 0.85)");
  aoGrad.addColorStop(1, "rgba(3, 1, 7, 0)");
  ctx.fillStyle = aoGrad;
  ctx.beginPath();
  ctx.ellipse(bodyCenterX, bodyCenterY - 3, 43, 19, 0, Math.PI, Math.PI * 2);
  ctx.fill();

  // Crisp 3D Bottom Rim Bounce Light
  ctx.strokeStyle = isBlocked
    ? "rgba(244, 63, 94, 0.4)"
    : "rgba(168, 85, 247, 0.42)";
  ctx.lineWidth = 1.8;
  ctx.beginPath();
  ctx.ellipse(
    bodyCenterX,
    bodyCenterY,
    43.2,
    23.2,
    0,
    0.12 * Math.PI,
    0.88 * Math.PI,
  );
  ctx.stroke();
  ctx.restore();

  // =======================================================================
  // 4. OUTER HOOD SILHOUETTE (Exact Original Shape + 3D Extruded Depth Layer)
  // =======================================================================
  const topPeakX = -px * 0.24;
  const topPeakY = -66 + py * 0.25;
  const leftBrimX = -76 + (lookX < 0 ? lookX * 10 : lookX * 16);
  const rightBrimX = 76 + (lookX > 0 ? lookX * 10 : lookX * 16);
  const brimY = 26 - py * 0.35;

  ctx.save();

  // 4A. 3D BACK EXTRUSION WALL (Gives real 3D thickness to the outer hood when turning)
  if (!isMiniIcon && depth3D > 0.6) {
    const extX = -lookX * 5.5 * depth3D;
    const extY = -lookY * 3.5 * depth3D - 1.5;
    ctx.fillStyle = "#120c1c";
    ctx.beginPath();
    ctx.moveTo(topPeakX + extX, topPeakY + extY);
    ctx.bezierCurveTo(
      38 + px * 0.15 + extX,
      -64 + py * 0.2 + extY,
      58 + px * 0.2 + extX,
      -12 + extY,
      rightBrimX + extX,
      brimY + extY,
    );
    ctx.bezierCurveTo(
      rightBrimX - 8 + extX,
      brimY + 18 + extY,
      35 + px * 0.3,
      38 + py * 0.4,
      px * 0.35,
      34 + py * 0.45,
    );
    ctx.bezierCurveTo(
      -35 + px * 0.3,
      38 + py * 0.4,
      leftBrimX + 8 + extX,
      brimY + 18 + extY,
      leftBrimX + extX,
      brimY + extY,
    );
    ctx.bezierCurveTo(
      -58 + px * 0.2 + extX,
      -12 + extY,
      -38 + px * 0.15 + extX,
      -64 + py * 0.2 + extY,
      topPeakX + extX,
      topPeakY + extY,
    );
    ctx.closePath();
    ctx.fill();
  }

  // 4B. MAIN SCULPTED HOOD SURFACE (Exact Silhouette + Enhanced 3D Volumetric Shader)
  const hoodGrad = ctx.createLinearGradient(
    -48 + px * 0.2,
    -68,
    58 + px * 0.2,
    48,
  );
  hoodGrad.addColorStop(0, "#43325c"); // Brighter 3D top-left key light
  hoodGrad.addColorStop(0.28, "#2b1f3d"); // Rich sculpted mid-violet
  hoodGrad.addColorStop(0.68, "#171122"); // Deep shadow fold
  hoodGrad.addColorStop(0.9, "#100b18"); // Dark obsidian base
  hoodGrad.addColorStop(1, "#2d1d42"); // Right crystal bounce light

  ctx.fillStyle = hoodGrad;
  ctx.beginPath();
  ctx.moveTo(topPeakX, topPeakY);
  ctx.bezierCurveTo(
    38 + px * 0.15,
    -64 + py * 0.2,
    58 + px * 0.2,
    -12,
    rightBrimX,
    brimY,
  );
  ctx.bezierCurveTo(
    rightBrimX - 8,
    brimY + 18,
    35 + px * 0.3,
    38 + py * 0.4,
    px * 0.35,
    34 + py * 0.45,
  );
  ctx.bezierCurveTo(
    -35 + px * 0.3,
    38 + py * 0.4,
    leftBrimX + 8,
    brimY + 18,
    leftBrimX,
    brimY,
  );
  ctx.bezierCurveTo(
    -58 + px * 0.2,
    -12,
    -38 + px * 0.15,
    -64 + py * 0.2,
    topPeakX,
    topPeakY,
  );
  ctx.closePath();
  ctx.fill();

  // 4C. 3D SPECULAR VOLUME HIGHLIGHT ON UPPER HOOD DOME (Reacts to 3D tilt & crystal glow)
  if (!isMiniIcon) {
    ctx.save();
    ctx.clip(); // Stay strictly inside the exact hood silhouette!

    // Soft 3D spherical sheen on upper-left crown of the hood
    const domeHighlight = ctx.createRadialGradient(
      -18 + px * 0.35,
      -42 + py * 0.35,
      2,
      -14 + px * 0.3,
      -36 + py * 0.3,
      46,
    );
    domeHighlight.addColorStop(0, "rgba(216, 180, 254, 0.18)");
    domeHighlight.addColorStop(0.6, "rgba(168, 85, 247, 0.06)");
    domeHighlight.addColorStop(1, "rgba(168, 85, 247, 0)");
    ctx.fillStyle = domeHighlight;
    ctx.beginPath();
    ctx.arc(-14 + px * 0.3, -36 + py * 0.3, 46, 0, Math.PI * 2);
    ctx.fill();

    // 3D Crystal Point-Light Bounce on Upper-Right Slope of Hood
    const crystalBounce = ctx.createRadialGradient(
      26 + px * 0.25,
      -50 + py * 0.25,
      2,
      22 + px * 0.25,
      -42 + py * 0.25,
      44,
    );
    const bounceCol = isBlocked
      ? "244, 63, 94"
      : isSuccess
        ? "250, 204, 21"
        : "232, 121, 249";
    crystalBounce.addColorStop(
      0,
      `rgba(${bounceCol}, ${0.28 * glowIntensity})`,
    );
    crystalBounce.addColorStop(1, `rgba(${bounceCol}, 0)`);
    ctx.fillStyle = crystalBounce;
    ctx.beginPath();
    ctx.arc(22 + px * 0.25, -42 + py * 0.25, 44, 0, Math.PI * 2);
    ctx.fill();

    // Subtle 3D Tailored Cloak Seam Lines curving from peak to brim
    ctx.strokeStyle = "rgba(12, 8, 20, 0.45)";
    ctx.lineWidth = 1.4;
    ctx.beginPath();
    ctx.moveTo(topPeakX, topPeakY);
    ctx.quadraticCurveTo(
      -28 + px * 0.45,
      -25 + py * 0.4,
      leftBrimX + 20,
      brimY + 4,
    );
    ctx.moveTo(topPeakX, topPeakY);
    ctx.quadraticCurveTo(
      28 + px * 0.45,
      -25 + py * 0.4,
      rightBrimX - 20,
      brimY + 4,
    );
    ctx.stroke();

    ctx.restore();
  }

  // Hood Side Fold Wings (Visible in Left/Right/Up/Down turns)
  if (Math.abs(lookX) > 0.25 || lookY > 0.25) {
    ctx.fillStyle = "#1b1426";
    const wingSide = lookX >= 0 ? -1 : 1;
    ctx.beginPath();
    ctx.moveTo(wingSide * 42, -38);
    ctx.quadraticCurveTo(wingSide * 66, -42, wingSide * 56, -24);
    ctx.closePath();
    ctx.fill();
  }

  // Outer Rim Highlight on Hood Edge (3D Fresnel Rim Light)
  ctx.strokeStyle = isBlocked
    ? `rgba(251, 113, 133, ${0.48 * glowIntensity})`
    : `rgba(216, 180, 254, ${0.48 * glowIntensity})`;
  ctx.lineWidth = 1.8;
  ctx.beginPath();
  ctx.moveTo(topPeakX, topPeakY);
  ctx.bezierCurveTo(
    38 + px * 0.15,
    -64 + py * 0.2,
    58 + px * 0.2,
    -12,
    rightBrimX,
    brimY,
  );
  ctx.stroke();

  // Left subtle rim highlight
  ctx.strokeStyle = "rgba(167, 139, 250, 0.28)";
  ctx.lineWidth = 1.3;
  ctx.beginPath();
  ctx.moveTo(topPeakX, topPeakY);
  ctx.bezierCurveTo(
    -38 + px * 0.15,
    -64 + py * 0.2,
    -58 + px * 0.2,
    -12,
    leftBrimX,
    brimY,
  );
  ctx.stroke();
  ctx.restore();

  // =======================================================================
  // 5. 3D INNER HOOD BEVEL WALL & DEEP VOID CAVITY
  // =======================================================================
  const openCenterX = px * 0.85;
  const openTopY = -28 + py * 0.95;
  const openBottomY = 32 + py * 0.45;
  const openHalfWidth = 46 - Math.abs(lookX) * 6;

  // 5A. 3D INNER HOOD THICKNESS BEVEL (The sculpted cloth thickness between outer trim & void)
  const bevelThickness = 4.2 * (0.7 + depth3D * 0.3);
  ctx.save();
  const bevelGrad = ctx.createLinearGradient(
    openCenterX,
    openTopY - bevelThickness,
    openCenterX,
    openBottomY,
  );
  bevelGrad.addColorStop(0, "#2e1d3a");
  bevelGrad.addColorStop(0.5, "#1a1024");
  bevelGrad.addColorStop(1, "#3b244b");
  ctx.fillStyle = bevelGrad;
  ctx.beginPath();
  ctx.moveTo(openCenterX - openHalfWidth - bevelThickness, openBottomY - 9);
  ctx.bezierCurveTo(
    openCenterX - openHalfWidth + 4,
    openTopY - bevelThickness * 1.3,
    openCenterX + openHalfWidth - 4,
    openTopY - bevelThickness * 1.3,
    openCenterX + openHalfWidth + bevelThickness,
    openBottomY - 9,
  );
  ctx.bezierCurveTo(
    openCenterX + openHalfWidth * 0.72,
    openBottomY + 13 + bevelThickness * 0.4,
    openCenterX - openHalfWidth * 0.72,
    openBottomY + 13 + bevelThickness * 0.4,
    openCenterX - openHalfWidth - bevelThickness,
    openBottomY - 9,
  );
  ctx.closePath();
  ctx.fill();
  ctx.restore();

  // Metallic Bronze / Rose-Gold 3D Seam Arch along the Hood Brow
  ctx.save();
  ctx.strokeStyle =
    outfit === "special" ? "#fbbf24" : "rgba(186, 132, 174, 0.75)";
  ctx.lineWidth = outfit === "special" ? 3.0 : 2.2;
  ctx.beginPath();
  ctx.moveTo(leftBrimX + 4, brimY - 1);
  ctx.bezierCurveTo(
    -36 + px * 0.65,
    -36 + py * 0.85,
    36 + px * 0.65,
    -36 + py * 0.85,
    rightBrimX - 4,
    brimY - 1,
  );
  ctx.stroke();
  ctx.restore();

  // 5B. DEEP BLACK 3D VOID OPENING
  ctx.save();
  ctx.beginPath();
  ctx.moveTo(openCenterX - openHalfWidth, openBottomY - 10);
  ctx.bezierCurveTo(
    openCenterX - openHalfWidth + 6,
    openTopY,
    openCenterX + openHalfWidth - 6,
    openTopY,
    openCenterX + openHalfWidth,
    openBottomY - 10,
  );
  ctx.bezierCurveTo(
    openCenterX + openHalfWidth * 0.7,
    openBottomY + 12,
    openCenterX - openHalfWidth * 0.7,
    openBottomY + 12,
    openCenterX - openHalfWidth,
    openBottomY - 10,
  );
  ctx.closePath();

  const voidGrad = ctx.createRadialGradient(
    openCenterX + px * 0.15,
    (openTopY + openBottomY) * 0.5,
    6,
    openCenterX,
    (openTopY + openBottomY) * 0.5,
    54,
  );
  voidGrad.addColorStop(0, "#0a0513");
  voidGrad.addColorStop(0.72, "#040208");
  voidGrad.addColorStop(1, "#150c24");
  ctx.fillStyle = voidGrad;
  ctx.fill();

  // Clip inside the hood cavity for eyes, inner 3D depth shading, and inner accessories
  ctx.clip();

  // 3D Inner Cavity Wall Rim Light (Shows the 3D curvature inside the dark hood)
  if (!isMiniIcon) {
    const innerBounce = ctx.createRadialGradient(
      openCenterX + px * 0.3,
      openBottomY + 4,
      4,
      openCenterX,
      openBottomY,
      44,
    );
    const innerCol = isBlocked ? "244, 63, 94" : "192, 132, 252";
    innerBounce.addColorStop(0, `rgba(${innerCol}, ${0.22 * glowIntensity})`);
    innerBounce.addColorStop(1, `rgba(${innerCol}, 0)`);
    ctx.fillStyle = innerBounce;
    ctx.fillRect(openCenterX - 60, openTopY - 10, 120, 80);
  }

  // Starry Outfit inner sparkles inside the void
  if (outfit === "starry" && !isMiniIcon) {
    ctx.fillStyle = "#fde047";
    for (let s = 0; s < 5; s++) {
      const sx = openCenterX + Math.sin(time * 1.5 + s * 1.7) * 32;
      const sy =
        (openTopY + openBottomY) * 0.5 + Math.cos(time * 1.2 + s * 2.1) * 16;
      const sr = (Math.sin(time * 4 + s) * 0.5 + 0.5) * 1.8;
      ctx.beginPath();
      ctx.arc(sx, sy, sr, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  // =======================================================================
  // 6. EXPRESSIVE GLOWING VIOLET EYES (All 9 Expressions + 3D Lens Highlight)
  // =======================================================================
  const eyeSpacing = 17.5 * (1 - Math.abs(lookX) * 0.22);
  const eyeBaseX = px * 1.12;
  const eyeBaseY = 4 + py * 0.82;

  const leftEyeX = eyeBaseX - eyeSpacing;
  const rightEyeX = eyeBaseX + eyeSpacing;

  const leftEyeScaleX =
    lookX < 0 ? 1 - Math.abs(lookX) * 0.28 : 1 + lookX * 0.06;
  const rightEyeScaleX =
    lookX > 0 ? 1 - Math.abs(lookX) * 0.28 : 1 - lookX * 0.06;

  drawExpressiveEye3D(
    ctx,
    leftEyeX,
    eyeBaseY,
    -1,
    leftEyeScaleX,
    lookX,
    lookY,
    state,
    outfit,
    blinkProgress,
    time,
    glowIntensity,
    isMiniIcon,
  );
  drawExpressiveEye3D(
    ctx,
    rightEyeX,
    eyeBaseY,
    1,
    rightEyeScaleX,
    lookX,
    lookY,
    state,
    outfit,
    blinkProgress,
    time,
    glowIntensity,
    isMiniIcon,
  );

  // Speaking State Cute Little Glowing Mouth Pulse
  if (state === "speaking") {
    const mouthOpen = 3.5 + Math.abs(Math.sin(time * 12)) * 4.5;
    ctx.save();
    ctx.translate(eyeBaseX, eyeBaseY + 13);
    ctx.fillStyle = "#f5d0fe";
    ctx.shadowColor = "#d946ef";
    ctx.shadowBlur = 10 * glowIntensity;
    ctx.beginPath();
    ctx.arc(0, 0, mouthOpen, 0, Math.PI, false);
    ctx.closePath();
    ctx.fill();
    ctx.restore();
  }

  // Mask Outfit (Shadow Ninja Lower Face Veil with 3D curved shading)
  if (outfit === "mask") {
    ctx.save();
    const maskGrad = ctx.createLinearGradient(
      openCenterX - 35,
      eyeBaseY + 6,
      openCenterX + 35,
      openBottomY + 14,
    );
    maskGrad.addColorStop(0, "#1e162e");
    maskGrad.addColorStop(0.5, "#2d2144");
    maskGrad.addColorStop(1, "#120d1c");
    ctx.fillStyle = maskGrad;
    ctx.beginPath();
    ctx.moveTo(openCenterX - 48, eyeBaseY + 10);
    ctx.quadraticCurveTo(
      eyeBaseX,
      eyeBaseY + 2,
      openCenterX + 48,
      eyeBaseY + 10,
    );
    ctx.lineTo(openCenterX + 48, openBottomY + 18);
    ctx.lineTo(openCenterX - 48, openBottomY + 18);
    ctx.closePath();
    ctx.fill();

    ctx.strokeStyle = "#c084fc";
    ctx.lineWidth = 1.7;
    ctx.beginPath();
    ctx.moveTo(openCenterX - 46, eyeBaseY + 10);
    ctx.quadraticCurveTo(
      eyeBaseX,
      eyeBaseY + 2,
      openCenterX + 46,
      eyeBaseY + 10,
    );
    ctx.stroke();
    ctx.restore();
  }

  ctx.restore(); // End inner hood cavity clip

  // Crisp 3D Inner Hood Rim Highlight Line framing the face cavity
  ctx.save();
  ctx.strokeStyle = "rgba(200, 145, 220, 0.65)";
  ctx.lineWidth = 1.9;
  ctx.beginPath();
  ctx.moveTo(openCenterX - openHalfWidth, openBottomY - 10);
  ctx.bezierCurveTo(
    openCenterX - openHalfWidth + 6,
    openTopY,
    openCenterX + openHalfWidth - 6,
    openTopY,
    openCenterX + openHalfWidth,
    openBottomY - 10,
  );
  ctx.stroke();
  ctx.restore();

  // =======================================================================
  // 7. BOTTOM DANGLING 3D CRYSTAL EARRING (With Pendulum Physics + 3D Spin)
  // =======================================================================
  const earringAnchorX = 34 + px * 0.45;
  const earringAnchorY = 30 + py * 0.35;
  ctx.save();
  ctx.translate(earringAnchorX, earringAnchorY);
  ctx.rotate(earringAngle - stateTilt * 0.8);

  // 3D Metallic Bronze attachment hoop
  const hoopGrad = ctx.createLinearGradient(-4, -4, 4, 4);
  hoopGrad.addColorStop(0, "#fde047");
  hoopGrad.addColorStop(0.6, "#d97706");
  hoopGrad.addColorStop(1, "#78350f");
  ctx.strokeStyle = hoopGrad;
  ctx.lineWidth = 2.4;
  ctx.beginPath();
  ctx.arc(0, 0, 3.6, 0, Math.PI * 2);
  ctx.stroke();

  // True 3D Rotating Faceted Purple Diamond Earring
  drawTrue3DCrystal(
    ctx,
    0,
    13,
    6.8,
    11.5,
    time * 1.8 + lookX * 1.5,
    lookY * 0.35,
    glowIntensity,
    isBlocked,
  );
  ctx.restore();

  // =======================================================================
  // 8. TOP SIGNATURE FLOATING 3D CRYSTAL + DEPTH-SORTED ORBITAL RING + STUD
  // =======================================================================
  const crystalAngleOffset = Math.abs(lookY) > 0.5 ? 0.15 : 0.55;
  const studX = 24 * crystalAngleOffset + px * 0.25;
  const studY = -52 + py * 0.25;

  ctx.save();
  ctx.translate(studX, studY);
  ctx.rotate(0.28 * crystalAngleOffset - lookX * 0.15);

  // 3D Sculpted Bronze/Gold Circular Hood Base Stud
  const studGrad = ctx.createRadialGradient(-2, -2, 0.5, 0, 0, 7);
  studGrad.addColorStop(0, "#fef08a");
  studGrad.addColorStop(0.5, "#d97706");
  studGrad.addColorStop(1, "#451a03");
  ctx.fillStyle = studGrad;
  ctx.beginPath();
  ctx.ellipse(0, 0, 6.2, 3.6, 0, 0, Math.PI * 2);
  ctx.fill();

  const crystalFloatY = -20 + Math.sin(time * 3.8) * 2.8;
  const ringTilt = -0.18 + Math.sin(time * 2) * 0.06;

  // 8A. BACK HALF OF 3D ORBITAL RING (Drawn BEHIND the 3D Crystal!)
  if (!isMiniIcon) {
    ctx.save();
    ctx.translate(0, crystalFloatY * 0.58);
    ctx.rotate(ringTilt);
    ctx.strokeStyle = isBlocked
      ? "rgba(244, 63, 94, 0.38)"
      : "rgba(192, 132, 252, 0.42)";
    ctx.lineWidth = 1.5;
    ctx.beginPath();
    ctx.ellipse(0, 0, 22, 6.5, 0, Math.PI, Math.PI * 2);
    ctx.stroke();
    ctx.restore();
  }

  // 8B. MAIN FLOATING TOP 3D DIAMOND CRYSTAL (Rotates in 3D space!)
  drawTrue3DCrystal(
    ctx,
    0,
    crystalFloatY,
    isMiniIcon ? 11 : 9.8,
    isMiniIcon ? 18 : 16.8,
    time * 1.45 + lookX * 1.4,
    0.18 + lookY * 0.35,
    glowIntensity,
    isBlocked,
  );

  // 8C. FRONT HALF OF 3D ORBITAL RING (Drawn IN FRONT OF the 3D Crystal!)
  if (!isMiniIcon) {
    ctx.save();
    ctx.translate(0, crystalFloatY * 0.58);
    ctx.rotate(ringTilt);
    ctx.strokeStyle = isBlocked
      ? "rgba(253, 164, 175, 0.88)"
      : "rgba(245, 208, 254, 0.88)";
    ctx.lineWidth = 2.0;
    ctx.shadowColor = isBlocked ? "#f43f5e" : "#d946ef";
    ctx.shadowBlur = 10 * glowIntensity;
    ctx.beginPath();
    ctx.ellipse(0, 0, 22, 6.5, 0, 0, Math.PI);
    ctx.stroke();

    // Tiny 3D Orbiting Energy Spark along the ring
    const orbAng = (time * 3.2) % (Math.PI * 2);
    const ox = Math.cos(orbAng) * 22;
    const oy = Math.sin(orbAng) * 6.5;
    if (Math.sin(orbAng) >= -0.2) {
      ctx.fillStyle = "#ffffff";
      ctx.beginPath();
      ctx.arc(ox, oy, 2.1, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }
  ctx.restore();

  // =======================================================================
  // 9. OUTFITS & ACCESSORIES RENDERER (10 Outfits with 3D Shading)
  // =======================================================================
  drawOutfitAccessory(
    ctx,
    outfit,
    px,
    py,
    leftBrimX,
    rightBrimX,
    brimY,
    topPeakX,
    topPeakY,
    time,
    glowIntensity,
    isMiniIcon,
  );

  // =======================================================================
  // 10. STATE-SPECIFIC VISUAL FX (Working Rings, Stars, ?, !, Warning, zZz)
  // =======================================================================
  if (!isMiniIcon) {
    drawStateEffects(
      ctx,
      state,
      time,
      glowIntensity,
      frame.sleepingMarks,
      frame.notificationPulse,
    );
  }

  // =======================================================================
  // 11. 3D FLOATING SHADOW PAWS / HANDS (Interactive & Window Perching)
  // =======================================================================
  if ((showPaws && !isMiniIcon) || perchingHands) {
    draw3DShadowPaws(
      ctx,
      px,
      py,
      bobY,
      state,
      time,
      perchingHands,
      glowIntensity,
    );
  }

  // =======================================================================
  // 12. FLOATING UI SPEECH BUBBLE
  // =======================================================================

  ctx.restore();
}
