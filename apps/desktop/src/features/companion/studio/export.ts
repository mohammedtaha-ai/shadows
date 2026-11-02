import type { CharacterFrame, CompanionSettings, OutfitId } from "../types.ts";
import { DEFAULT_SETTINGS } from "../catalog.ts";
import { renderCharacter } from "../renderer/index.ts";

export async function exportOutfitPreview(outfit: OutfitId): Promise<Blob> {
  const source = await exportCharacterPNG({
    cx: 0, cy: 0, scale: 1, lookX: 0, lookY: 0, tilt: 0, time: 0,
    earringAngle: 0, blinkProgress: 0, expression: "idle", outfit,
    mini: false, perchingHands: false, sleepingMarks: [], notificationPulse: 0,
  }, DEFAULT_SETTINGS);
  const image = await createImageBitmap(source);
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 160;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("PNG preview context unavailable");
  context.drawImage(image, 0, 0, 160, 160);
  image.close();
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error("PNG preview failed")), "image/png");
  });
}
export async function exportCharacterPNG(
  frame: CharacterFrame,
  settings: CompanionSettings,
): Promise<Blob> {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 512;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("PNG export context unavailable");
  const frozen = structuredClone(frame);
  renderCharacter(
    ctx,
    {
      ...frozen,
      cx: 256,
      cy: 275,
      scale: 1.55,
      mini: false,
      perchingHands: false,
      notificationPulse: 0,
    },
    settings,
  );
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error("PNG export failed"))),
      "image/png",
    );
  });
}
export function downloadCharacterPNG(blob: Blob, filename: string): void {
  const url = URL.createObjectURL(blob),
    link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  try {
    link.click();
  } finally {
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
}
