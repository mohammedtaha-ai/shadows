import React from "react";
import type {
  CompanionMessage,
  CompanionSettings,
  DirectionId,
  Expression,
  MotionInput,
  OutfitId,
  Rect,
  SceneFrame,
  SceneMode,
} from "../types.ts";
import { DEFAULT_SETTINGS, normalizeSettings } from "../catalog.ts";
import { CompanionCanvas } from "../CompanionCanvas";
import { SpeechBubble } from "../SpeechBubble";
import { createMotion, readFrame } from "../animation/model.ts";
import { createTones, type ToneController } from "../audio.ts";
import { SceneControls } from "./SceneControls";
import { PreviewGrid } from "./PreviewGrid";
import { MockWindow } from "./MockWindow";
import { createSimulation, type Simulation } from "./simulation.ts";
import { exportCharacterPNG, downloadCharacterPNG } from "./export.ts";
import "../companion.css";
import "./studio.css";
export function CompanionStudio(): React.JSX.Element {
  const [expression, setExpression] = React.useState<Expression>("idle"),
    [outfit, setOutfit] = React.useState<OutfitId>("default"),
    [scene, setScene] = React.useState<SceneMode>("studio");
  const [settings, setSettings] =
      React.useState<CompanionSettings>(DEFAULT_SETTINGS),
    [direction, setDirection] = React.useState<DirectionId | null>(null),
    [message, setMessage] = React.useState<CompanionMessage | null>(null),
    [paused, setPaused] = React.useState(false),
    [sound, setSound] = React.useState(false),
    [error, setError] = React.useState(""),
    [text, setText] = React.useState(""),
    [perch, setPerch] = React.useState<Rect | null>(null);
  const simulation = React.useRef<Simulation | null>(null),
    tones = React.useRef<ToneController | null>(null),
    snapshot = React.useRef<{
      frame: SceneFrame;
      direction: DirectionId | null;
    } | null>(null);
  React.useEffect(() => {
    const audio = createTones();
    tones.current = audio;
    const sim = createSimulation(
      {
        set: (fn, ms) => window.setTimeout(fn, ms),
        clear: (id) => window.clearTimeout(id),
      },
      (update) => {
        if (update.expression) setExpression(update.expression);
        if ("message" in update) setMessage(update.message ?? null);
        if (update.outfit) setOutfit(update.outfit);
        if (update.tone) audio.play(update.tone);
      },
    );
    simulation.current = sim;
    return () => {
      sim.dispose();
      audio.dispose();
      simulation.current = null;
      tones.current = null;
    };
  }, []);
  const selectExpression = (v: Expression) => {
    simulation.current?.cancel();
    setExpression(v);
    setMessage(null);
  };
  const input: MotionInput = {
    viewport: { width: 400, height: 360, dpr: 1 },
    presentation: { expression, statusText: expression, message },
    settings,
    outfit,
    scene,
    pointer: null,
    direction,
    perch,
  };
  const staticFrame = React.useMemo(
    () =>
      readFrame(
        createMotion(input.viewport, () => 0.5),
        input,
      ).character,
    [expression, outfit, direction, settings],
  );
  return (
    <section
      className="aylul-studio"
      role="region"
      aria-label="AYLUL development studio"
    >
      <h2>AYLUL development studio</h2>
      <p>Visual previews and synthetic commands only.</p>
      <div className="aylul-studio-layout">
        <div>
          <div className="aylul-studio-stage">
            <CompanionCanvas
              input={input}
              paused={paused}
              onFrame={(frame) => {
                snapshot.current = { frame, direction };
              }}
              onFailure={setError}
              onPoke={() => tones.current?.play("poke")}
            />
            {scene === "window" && <MockWindow onPerch={setPerch} />}
          </div>
          <SpeechBubble message={message} />
          <p role="status" aria-label="Preview expression">
            {expression}
          </p>
          <div className="aylul-controls">
            <button type="button" onClick={() => setPaused((v) => !v)}>
              {paused ? "Resume preview" : "Pause preview"}
            </button>
            <button
              type="button"
              onClick={() => {
                if (sound) {
                  tones.current?.disable();
                  setSound(false);
                } else
                  void tones.current
                    ?.enable()
                    .then(() => setSound(true))
                    .catch(() => setError("Preview sound unavailable."));
              }}
            >
              {sound ? "Mute preview sound" : "Enable preview sound"}
            </button>
            <button type="button" onClick={() => simulation.current?.notify()}>
              Preview notification
            </button>
            <button
              type="button"
              onClick={() => simulation.current?.run("success")}
            >
              Simulate success
            </button>
            <button
              type="button"
              onClick={() => simulation.current?.run("blocked")}
            >
              Simulate blocked
            </button>
            <button
              type="button"
              onClick={() => simulation.current?.run("sleep")}
            >
              Simulate sleep
            </button>
            <button
              type="button"
              onClick={() => {
                setError("");
                void exportCharacterPNG(
                  {
                    ...(snapshot.current?.frame.character ?? staticFrame),
                    expression,
                    outfit,
                    ...(direction || snapshot.current?.direction !== direction
                      ? {
                          lookX: staticFrame.lookX,
                          lookY: staticFrame.lookY,
                          tilt: staticFrame.tilt,
                        }
                      : {}),
                  },
                  settings,
                )
                  .then((blob) =>
                    downloadCharacterPNG(
                      blob,
                      `AYLUL_${expression}_${outfit}.png`,
                    ),
                  )
                  .catch(() => setError("PNG export failed."));
              }}
            >
              Export PNG
            </button>
          </div>
          {scene === "assistant" && (
            <form
              onSubmit={(e) => {
                e.preventDefault();
                simulation.current?.run(
                  /error|خطأ/i.test(text) ? "blocked" : "success",
                );
              }}
            >
              <label>
                Synthetic command
                <input value={text} onChange={(e) => setText(e.target.value)} />
              </label>
              <button>Preview command</button>
            </form>
          )}
          {error && <p role="alert">{error}</p>}
          <PreviewGrid frame={staticFrame} settings={settings} />
        </div>
        <SceneControls
          expression={expression}
          outfit={outfit}
          scene={scene}
          settings={settings}
          onExpression={selectExpression}
          onOutfit={(v) => {
            simulation.current?.cancel();
            setOutfit(v);
          }}
          onScene={(v) => {
            simulation.current?.cancel();
            setMessage(null);
            setScene(v);
          }}
          onDirection={setDirection}
          onSettings={(v) =>
            setSettings((old) => normalizeSettings({ ...old, ...v }))
          }
        />
      </div>
    </section>
  );
}
