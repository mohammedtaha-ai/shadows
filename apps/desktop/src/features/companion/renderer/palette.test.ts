import assert from 'node:assert/strict';
import { test } from 'node:test';
import table from '../../../../../../assets/aylul/character-colors.json' with { type: 'json' };
import { paletteFor, rgba } from './palette.ts';

const keys = [
  'glow', 'glowSoft', 'glowPale', 'glowMist', 'rimLeft', 'spark', 'sparkDeep',
  'sparkSoft', 'sparkPale', 'core', 'bloom', 'crystalDeep', 'crystalDark', 'innerRim',
  'sleep', 'sleepGlow', 'bodyLit', 'bodyMid', 'bodyCore', 'bodyRim', 'occlusion',
  'extrusion', 'hoodLit', 'hoodMid', 'hoodFold', 'hoodBase', 'hoodBounce', 'seamLine',
  'wing', 'bevelTop', 'bevelMid', 'bevelBottom', 'voidCore', 'voidDeep', 'voidRim',
  'pawLit', 'pawMid', 'pawDark', 'maskLit', 'maskMid', 'maskDark', 'brow', 'floor',
] as const;

// Independent RGB-to-HSL measurement of the returned 8-bit colours.
function measure(hex: string) {
  const rgb = hex.slice(1).match(/../g)!.map(value => parseInt(value, 16));
  const max = Math.max(...rgb), min = Math.min(...rgb), chroma = max - min;
  const lightness = (max + min) / 510;
  const saturation = chroma ? chroma / (255 - Math.abs(max + min - 255)) : 0;
  let hue = 0;
  if (chroma) {
    const [r, g, b] = rgb;
    hue = max === r ? (g - b) / chroma : max === g ? (b - r) / chroma + 2 : (r - g) / chroma + 4;
    hue = (hue * 60 + 360) % 360;
  }
  return { hue, saturation, lightness, chroma };
}

test("the base palette is today's colours", () => {
  assert.deepEqual(paletteFor('purple'), table.colors.find(color => color.id === 'purple')!.tones);
  assert.deepEqual(Object.keys(paletteFor('purple')).sort(), [...keys].sort());
});

test('an unknown or missing id draws purple', () => {
  for (const id of ['red', 'teal', '', null, undefined]) {
    assert.strictEqual(paletteFor(id), paletteFor('purple'));
  }
});

test('every colour in the table has a full palette', () => {
  for (const entry of table.colors) {
    const palette = paletteFor(entry.id);
    assert.strictEqual(paletteFor(entry.id), palette);
    assert.deepEqual(Object.keys(palette).sort(), [...keys].sort());
    for (const tone of Object.values(palette)) assert.match(tone, /^#[0-9a-f]{6}$/i);
    if (entry.id !== table.base) assert.notEqual(palette.glow, paletteFor(table.base).glow);
  }
});

test('a derived tone keeps lightness and saturation', () => {
  const baseEntry = table.colors.find(entry => entry.id === table.base)!;
  for (const entry of table.colors.filter(entry => entry.id !== table.base)) {
    const delta = measure(entry.swatch).hue - measure(baseEntry.swatch).hue;
    for (const [key, tone] of Object.entries(paletteFor(entry.id)) as [keyof ReturnType<typeof paletteFor>, string][]) {
      if (entry.tones && key in entry.tones) continue;
      const base = measure(paletteFor(table.base)[key]);
      const actual = measure(tone);
      assert.ok(Math.abs(actual.lightness - base.lightness) <= 0.01, `${entry.id}.${key} lightness`);
      assert.ok(Math.abs(actual.saturation - base.saturation) <= 0.01, `${entry.id}.${key} saturation`);
      if (base.chroma >= 32) {
        const expectedHue = (base.hue + delta + 360) % 360;
        const distance = Math.abs(actual.hue - expectedHue);
        assert.ok(Math.min(distance, 360 - distance) <= 2, `${entry.id}.${key} hue`);
      }
    }
  }
});

test('an override wins', () => {
  const custom = { ...table, colors: [...table.colors, { id: 'x', label: 'X', swatch: '#3b82f6', tones: { glow: '#123456' } }] };
  const palette = paletteFor('x', custom);
  assert.equal(palette.glow, '#123456');
  for (const key of keys.filter(key => key !== 'glow')) assert.equal(palette[key], paletteFor('blue')[key]);
  assert.strictEqual(paletteFor('x', custom), palette);
  assert.strictEqual(paletteFor('x'), paletteFor('purple'));
});

test("rgba keeps today's text", () => {
  assert.equal(rgba('#a855f7', 0.42), 'rgba(168, 85, 247, 0.42)');
});

test("the table's entries are complete", () => {
  assert.equal(new Set(table.colors.map(entry => entry.id)).size, table.colors.length);
  for (const entry of table.colors) {
    assert.match(entry.id, /^[a-z][a-z0-9-]{0,31}$/);
    assert.ok(entry.label.trim());
    assert.match(entry.swatch, /^#[0-9a-f]{6}$/i);
  }
  assert.ok(table.colors.find(entry => entry.id === table.base)?.tones);
});
