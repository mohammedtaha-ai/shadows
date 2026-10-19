import type { Point, Rect, Viewport } from "../types.ts";
export function normalizeViewport(v: Viewport): Viewport {
  const dimension = (n: number) => (Number.isFinite(n) ? Math.max(0, n) : 0);
  return {
    width: dimension(v.width),
    height: dimension(v.height),
    dpr: Number.isFinite(v.dpr) && v.dpr > 0 ? v.dpr : 1,
  };
}
export function backingSize(v: Viewport): { width: number; height: number } {
  const n = normalizeViewport(v);
  return {
    width: Math.round(n.width * n.dpr),
    height: Math.round(n.height * n.dpr),
  };
}
export function fitCharacter(v: Viewport): { center: Point; scale: number } {
  const n = normalizeViewport(v);
  return {
    center: { x: n.width / 2, y: n.height * 0.53 },
    scale: Math.max(0, Math.min(n.width / 340, n.height / 360)),
  };
}
export function localPointer(client: Point, bounds: Rect): Point | null {
  const x = client.x - bounds.x,
    y = client.y - bounds.y;
  return Number.isFinite(x) &&
    Number.isFinite(y) &&
    x >= 0 &&
    y >= 0 &&
    x <= bounds.width &&
    y <= bounds.height
    ? { x, y }
    : null;
}
