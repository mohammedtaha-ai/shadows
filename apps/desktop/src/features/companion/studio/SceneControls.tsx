import React from "react";
import { EXPRESSIONS, OUTFITS, DIRECTIONS } from "../catalog.ts";
import type {
  CompanionSettings,
  DirectionId,
  Expression,
  OutfitId,
  SceneMode,
} from "../types.ts";
type Props = {
  expression: Expression;
  outfit: OutfitId;
  scene: SceneMode;
  settings: CompanionSettings;
  onExpression: (v: Expression) => void;
  onOutfit: (v: OutfitId) => void;
  onScene: (v: SceneMode) => void;
  onDirection: (v: DirectionId | null) => void;
  onSettings: (v: Partial<CompanionSettings>) => void;
};
export function SceneControls(p: Props): React.JSX.Element {
  return (
    <div className="aylul-studio-controls">
      <fieldset>
        <legend>Expressions</legend>
        {EXPRESSIONS.map((v) => (
          <button
            type="button"
            key={v.id}
            aria-pressed={v.id === p.expression}
            onClick={() => p.onExpression(v.id)}
          >
            {v.label}
          </button>
        ))}
      </fieldset>
      <fieldset>
        <legend>Outfits</legend>
        {OUTFITS.map((v) => (
          <button
            type="button"
            key={v.id}
            aria-pressed={v.id === p.outfit}
            onClick={() => p.onOutfit(v.id)}
          >
            {v.label}
          </button>
        ))}
      </fieldset>
      <label>
        Scene
        <select
          value={p.scene}
          onChange={(e) => p.onScene(e.target.value as SceneMode)}
        >
          {["studio", "cursor", "window", "assistant"].map((v) => (
            <option key={v}>{v}</option>
          ))}
        </select>
      </label>
      <fieldset>
        <legend>Look direction</legend>
        {DIRECTIONS.map((v) => (
          <button type="button" key={v.id} onClick={() => p.onDirection(v.id)}>
            {v.id}
          </button>
        ))}
        <button type="button" onClick={() => p.onDirection(null)}>
          Follow pointer
        </button>
      </fieldset>
      {(
        [
          ["depth3D", "Depth", 0.5, 2, 0.05],
          ["glowIntensity", "Glow", 0.5, 1.9, 0.05],
          ["floatSpeed", "Float speed", 0.3, 2.5, 0.1],
          ["earringSwing", "Crystal swing", 0.3, 2.2, 0.1],
        ] as const
      ).map(([key, label, min, max, step]) => (
        <label key={key}>
          {label}
          <input
            type="range"
            min={min}
            max={max}
            step={step}
            value={p.settings[key]}
            onChange={(e) => p.onSettings({ [key]: Number(e.target.value) })}
          />
        </label>
      ))}
      {(
        [
          ["showHands", "Hands"],
          ["showPedestal", "Pedestal"],
          ["followPointer", "Track pointer"],
        ] as const
      ).map(([key, label]) => (
        <label key={key}>
          <input
            type="checkbox"
            checked={p.settings[key]}
            onChange={(e) => p.onSettings({ [key]: e.target.checked })}
          />
          {label}
        </label>
      ))}
    </div>
  );
}
