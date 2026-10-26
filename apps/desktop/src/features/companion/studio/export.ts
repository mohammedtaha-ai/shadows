import type { CharacterFrame, CompanionSettings } from "../types.ts";
import { renderCharacter } from "../renderer/index.ts";
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
