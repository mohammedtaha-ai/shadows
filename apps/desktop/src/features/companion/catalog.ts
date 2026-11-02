import type {
  CompanionSettings,
  DirectionId,
  Expression,
  OutfitId,
} from "./types.ts";
export const EXPRESSIONS: readonly { id: Expression; label: string }[] = [
  { id: "idle", label: "Idle" },
  { id: "happy", label: "Happy" },
  { id: "listening", label: "Listening" },
  { id: "speaking", label: "Speaking" },
  { id: "thinking", label: "Thinking" },
  { id: "working", label: "Working" },
  { id: "success", label: "Success" },
  { id: "blocked", label: "Blocked" },
  { id: "sleeping", label: "Sleeping" },
];
export const OUTFITS: readonly { id: OutfitId; label: string }[] = [
  { id: "default", label: "Default" },
  { id: "crown", label: "Crown" },
  { id: "moon", label: "Moon" },
  { id: "scarf", label: "Scarf" },
  { id: "starry", label: "Starry" },
  { id: "winter", label: "Winter" },
  { id: "headphones", label: "Headphones" },
  { id: "mask", label: "Mask" },
  { id: "halo", label: "Halo" },
  { id: "special", label: "Special" },
];
export function outfitOf(id: string | null | undefined): OutfitId {
  return OUTFITS.find(outfit => outfit.id === id)?.id ?? 'default';
}
export const DIRECTIONS: readonly {
  id: DirectionId;
  lx: number;
  ly: number;
  tilt: number;
}[] = [
  { id: "front", lx: 0, ly: 0, tilt: 0 },
  { id: "left", lx: -0.82, ly: 0, tilt: -0.05 },
  { id: "right", lx: 0.82, ly: 0, tilt: 0.05 },
  { id: "up", lx: 0, ly: -0.78, tilt: 0 },
  { id: "down", lx: 0, ly: 0.78, tilt: 0 },
  { id: "tilt", lx: -0.18, ly: 0.08, tilt: -0.32 },
];
export const DEFAULT_SETTINGS: CompanionSettings = Object.freeze({
  depth3D: 1.35,
  glowIntensity: 1.25,
  floatSpeed: 1,
  earringSwing: 1.25,
  showHands: true,
  showPedestal: true,
  followPointer: true,
});
export function normalizeSettings(
  partial: Partial<CompanionSettings>,
): CompanionSettings {
  const numeric = (
    key: "depth3D" | "glowIntensity" | "floatSpeed" | "earringSwing",
    min: number,
    max: number,
  ) =>
    Number.isFinite(partial[key])
      ? Math.min(max, Math.max(min, partial[key]!))
      : DEFAULT_SETTINGS[key];
  return {
    depth3D: numeric("depth3D", 0.5, 2),
    glowIntensity: numeric("glowIntensity", 0.5, 1.9),
    floatSpeed: numeric("floatSpeed", 0.3, 2.5),
    earringSwing: numeric("earringSwing", 0.3, 2.2),
    showHands: partial.showHands ?? true,
    showPedestal: partial.showPedestal ?? true,
    followPointer: partial.followPointer ?? true,
  };
}
