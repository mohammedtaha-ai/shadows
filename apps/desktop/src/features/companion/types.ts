export type Expression =
  | "idle"
  | "happy"
  | "listening"
  | "speaking"
  | "thinking"
  | "working"
  | "success"
  | "blocked"
  | "sleeping";
export type OutfitId =
  | "default"
  | "crown"
  | "moon"
  | "scarf"
  | "starry"
  | "winter"
  | "headphones"
  | "mask"
  | "halo"
  | "special";
export type DirectionId = "front" | "left" | "right" | "up" | "down" | "tilt";
export type SceneMode = "studio" | "cursor" | "window" | "assistant";
export type Point = Readonly<{ x: number; y: number }>;
export type Rect = Readonly<{
  x: number;
  y: number;
  width: number;
  height: number;
}>;
export type Viewport = Readonly<{ width: number; height: number; dpr: number }>;
export type CompanionMessage = Readonly<{
  id: string;
  text: string;
  durationMs?: number;
}>;
export type CompanionPresentation = Readonly<{
  expression: Expression;
  statusText: string;
  message: CompanionMessage | null;
}>;
export type CompanionSettings = Readonly<{
  depth3D: number;
  glowIntensity: number;
  floatSpeed: number;
  earringSwing: number;
  showHands: boolean;
  showPedestal: boolean;
  followPointer: boolean;
}>;
export type Particle = Readonly<{
  x: number;
  y: number;
  z: number;
  radius: number;
  alpha: number;
  speedY: number;
  phase: number;
}>;
export type SleepingMark = Readonly<{ offset: number; size: number }>;
export type CharacterFrame = Readonly<{
  color?: string;
  cx: number;
  cy: number;
  scale: number;
  lookX: number;
  lookY: number;
  tilt: number;
  time: number;
  earringAngle: number;
  blinkProgress: number;
  expression: Expression;
  outfit: OutfitId;
  mini: boolean;
  perchingHands: boolean;
  sleepingMarks: readonly SleepingMark[];
  notificationPulse: number;
}>;
export type SceneFrame = Readonly<{
  character: CharacterFrame;
  particles: readonly Particle[];
}>;
export type MotionInput = Readonly<{
  color?: string;
  viewport: Viewport;
  presentation: CompanionPresentation;
  settings: CompanionSettings;
  outfit: OutfitId;
  scene: SceneMode;
  pointer: Point | null;
  direction: DirectionId | null;
  perch: Rect | null;
}>;
