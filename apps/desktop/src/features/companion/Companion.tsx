import React from "react";
import type {
  CompanionMessage,
  CompanionPresentation,
  MotionInput,
} from "./types.ts";
import { DEFAULT_SETTINGS, outfitOf } from "./catalog.ts";
import { CompanionCanvas } from "./CompanionCanvas";
import { SpeechBubble } from "./SpeechBubble";
import {
  createMessageSlot,
  receiveMessage,
  visibleMessage,
} from "./animation/index.ts";
import { createTones, type ToneController } from "./audio.ts";
import "./companion.css";
const idle: CompanionPresentation = {
  expression: "idle",
  statusText: "Idle",
  message: null,
};
export function Companion({
  presentation = idle,
  outfit = 'default',
}: {
  presentation?: CompanionPresentation;
  outfit?: string;
}): React.JSX.Element {
  const [paused, setPaused] = React.useState(false),
    [sound, setSound] = React.useState(false),
    [error, setError] = React.useState(""),
    [audioError, setAudioError] = React.useState("");
  const [message, setMessage] = React.useState<CompanionMessage | null>(null),
    slot = React.useRef(createMessageSlot());
  const tones = React.useRef<ToneController | null>(null);
  React.useEffect(() => {
    tones.current = createTones();
    return () => {
      tones.current?.dispose();
      tones.current = null;
    };
  }, []);
  React.useEffect(() => {
    receiveMessage(slot.current, presentation.message, Date.now());
    let timer: ReturnType<typeof setTimeout>;
    const refresh = () => {
      clearTimeout(timer);
      const now = Date.now();
      const value = visibleMessage(slot.current, now);
      setMessage(value);
      if (value)
        timer = setTimeout(refresh, Math.max(0, slot.current.deadline - now));
    };
    refresh();
    document.addEventListener("visibilitychange", refresh);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", refresh);
    };
  }, [presentation.message]);
  const input: MotionInput = {
    viewport: { width: 0, height: 0, dpr: 1 },
    presentation,
    settings: DEFAULT_SETTINGS,
    outfit: outfitOf(outfit),
    scene: "studio",
    pointer: null,
    direction: null,
    perch: null,
  };
  return (
    <div className="aylul-presence" aria-label="AYLUL companion">
      {error ? (
        <p role="alert">{error}</p>
      ) : (
        <CompanionCanvas
          input={input}
          paused={paused}
          onFailure={setError}
          onPoke={() => tones.current?.play("poke")}
        />
      )}
      <SpeechBubble message={message} />
      <p className="aylul-status" role="status" aria-label="AYLUL status">
        {presentation.statusText}
      </p>
      <div className="aylul-controls">
        <button type="button" onClick={() => setPaused((v) => !v)}>
          {paused ? "Resume motion" : "Pause motion"}
        </button>
        <button
          type="button"
          onClick={() => {
            setAudioError("");
            if (sound) {
              tones.current?.disable();
              setSound(false);
            } else
              void tones.current
                ?.enable()
                .then(() => setSound(true))
                .catch(() => {
                  setSound(false);
                  setAudioError("Sound is unavailable.");
                });
          }}
        >
          {sound ? "Mute sound" : "Enable sound"}
        </button>
      </div>
      {audioError && <p role="status">{audioError}</p>}
    </div>
  );
}
