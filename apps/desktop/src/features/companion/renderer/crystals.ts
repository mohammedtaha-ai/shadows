import type {
  CharacterFrame,
  CompanionSettings,
  Expression,
  OutfitId,
  SleepingMark,
} from "../types.ts";
export function drawTrue3DCrystal(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  halfW: number,
  halfH: number,
  rotY: number,
  tiltX: number,
  glowIntensity: number,
  isBlocked: boolean,
): void {
  ctx.save();
  ctx.translate(x, y);

  const glowCol = isBlocked ? "#f43f5e" : "#d946ef";
  ctx.shadowColor = glowCol;
  ctx.shadowBlur = 18 * glowIntensity;

  // 6 3D Vertices of an Octahedron (Top, Bottom, and 4 Equatorial Ring vertices)
  const baseRing = [
    { x: -1, y: -0.08, z: 0 },
    { x: 0, y: -0.08, z: 1 },
    { x: 1, y: -0.08, z: 0 },
    { x: 0, y: -0.08, z: -1 },
  ];
  const topV = { x: 0, y: -1, z: 0 };
  const botV = { x: 0, y: 1, z: 0 };

  const cosY = Math.cos(rotY);
  const sinY = Math.sin(rotY);
  const cosX = Math.cos(tiltX);
  const sinX = Math.sin(tiltX);

  function project3DVertex(v: { x: number; y: number; z: number }) {
    // Rotate around Y
    const rx = v.x * cosY + v.z * sinY;
    const rz = -v.x * sinY + v.z * cosY;
    const ry = v.y;
    // Rotate around X
    const fy = ry * cosX - rz * sinX;
    const fz = ry * sinX + rz * cosX;
    return {
      x: rx * halfW * 1.08,
      y: fy * halfH,
      z: fz,
    };
  }

  const pTop = project3DVertex(topV);
  const pBot = project3DVertex(botV);
  const pRing = baseRing.map(project3DVertex);

  // Build 8 triangular faces (4 upper, 4 lower)
  const faces = [];
  for (let i = 0; i < 4; i++) {
    const next = (i + 1) % 4;
    faces.push({
      v0: pTop,
      v1: pRing[i],
      v2: pRing[next],
      isUpper: true,
      idx: i,
    });
    faces.push({
      v0: pBot,
      v1: pRing[next],
      v2: pRing[i],
      isUpper: false,
      idx: i,
    });
  }

  // Painter's algorithm sort by average Z (back to front)
  faces.sort((a, b) => {
    const za = (a.v0.z + a.v1.z + a.v2.z) / 3;
    const zb = (b.v0.z + b.v1.z + b.v2.z) / 3;
    return za - zb;
  });

  // Palette for 3D crystal facets depending on normal lighting
  const purpleShadesUpper = ["#fdf4ff", "#f5d0fe", "#e879f9", "#c084fc"];
  const purpleShadesLower = ["#a855f7", "#9333ea", "#7e22ce", "#581c87"];
  const redShadesUpper = ["#fff1f2", "#fecdd3", "#fda4af", "#fb7185"];
  const redShadesLower = ["#f43f5e", "#e11d48", "#be123c", "#881337"];

  faces.forEach((f) => {
    // Compute 2D cross product for face visibility / lighting intensity
    const ux = f.v1.x - f.v0.x;
    const uy = f.v1.y - f.v0.y;
    const vx = f.v2.x - f.v0.x;
    const vy = f.v2.y - f.v0.y;
    const cross = ux * vy - uy * vx;

    // Only draw front-facing facets (or slightly translucent back facets)
    if (cross < -2) return;

    // Compute normal X component to pick specular shade
    const lightIndex = Math.min(
      3,
      Math.max(0, Math.floor(((f.v1.x + f.v2.x) / (halfW * 2) + 1) * 2)),
    );
    let fillCol;
    if (isBlocked) {
      fillCol = f.isUpper
        ? redShadesUpper[lightIndex]
        : redShadesLower[lightIndex];
    } else {
      fillCol = f.isUpper
        ? purpleShadesUpper[lightIndex]
        : purpleShadesLower[lightIndex];
    }

    ctx.beginPath();
    ctx.moveTo(f.v0.x, f.v0.y);
    ctx.lineTo(f.v1.x, f.v1.y);
    ctx.lineTo(f.v2.x, f.v2.y);
    ctx.closePath();
    ctx.fillStyle = fillCol;
    ctx.fill();

    // Crisp 3D facet edge highlight
    ctx.shadowBlur = 0;
    ctx.strokeStyle = f.isUpper
      ? "rgba(255, 255, 255, 0.78)"
      : "rgba(240, 171, 252, 0.38)";
    ctx.lineWidth = 0.95;
    ctx.stroke();
  });

  ctx.restore();
}
