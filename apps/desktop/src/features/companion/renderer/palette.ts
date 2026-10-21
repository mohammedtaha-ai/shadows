import colors from '../../../../../../assets/aylul/character-colors.json' with { type: 'json' };

export type Tone = keyof NonNullable<(typeof colors.colors)[number]['tones']>;
export type Palette = Readonly<Record<Tone, string>>;
type ColorTable = {
  base: string;
  colors: { id: string; label: string; swatch: string; tones?: Partial<Record<Tone, string>> }[];
};

const tables = new WeakMap<ColorTable, Map<string, Palette>>();

function rgb(hex: string): number[] {
  return [1, 3, 5].map(start => parseInt(hex.slice(start, start + 2), 16));
}

function hsl(hex: string): [number, number, number] {
  const [r, g, b] = rgb(hex).map(channel => channel / 255);
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  const delta = max - min, lightness = (max + min) / 2;
  if (!delta) return [0, 0, lightness];
  const sector = max === r ? (g - b) / delta : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4;
  return [(sector * 60 + 360) % 360, delta / (1 - Math.abs(2 * lightness - 1)), lightness];
}

function rotate(hex: string, delta: number): string {
  const [hue, saturation, lightness] = hsl(hex);
  const h = ((hue + delta) % 360 + 360) % 360 / 60;
  const chroma = (1 - Math.abs(2 * lightness - 1)) * saturation;
  const x = chroma * (1 - Math.abs(h % 2 - 1));
  const sectors = [[chroma, x, 0], [x, chroma, 0], [0, chroma, x], [0, x, chroma], [x, 0, chroma], [chroma, 0, x]];
  return '#' + sectors[Math.floor(h)].map(channel =>
    Math.round((channel + lightness - chroma / 2) * 255).toString(16).padStart(2, '0'),
  ).join('');
}

export function paletteFor(id: string | null | undefined, table: ColorTable = colors): Palette {
  let cache = tables.get(table);
  if (!cache) {
    cache = new Map();
    tables.set(table, cache);
  }
  const base = table.colors.find(entry => entry.id === table.base)!;
  const entry = table.colors.find(entry => entry.id === id) ?? base;
  const cached = cache.get(entry.id);
  if (cached) return cached;
  const delta = hsl(entry.swatch)[0] - hsl(base.swatch)[0];
  const tones = entry === base ? { ...base.tones } : {
    ...Object.fromEntries(Object.entries(base.tones!).map(([tone, hex]) => [tone, rotate(hex, delta)])),
    ...entry.tones,
  };
  const palette = Object.freeze(tones) as Palette;
  cache.set(entry.id, palette);
  return palette;
}

export function rgba(tone: string, alpha: number): string {
  return `rgba(${rgb(tone).join(', ')}, ${alpha})`;
}
