import React from "react";
import type { MotionInput, SceneFrame, Point } from "./types.ts";
import {
  createMotion,
  advanceMotion,
  pokeMotion,
  readFrame,
  createFrameLoop,
  type FrameLoop,
  backingSize,
  localPointer,
} from "./animation/index.ts";
import { renderScene } from "./renderer/index.ts";
export type CompanionCanvasProps = {
  input: MotionInput;
  paused: boolean;
  renderAllowed?: boolean;
  onReady?: () => void;
  actionLabel?: string;
  onFrame?: (frame: SceneFrame) => void;
  onFailure: (reason: string) => void;
  onPoke?: () => void;
};
export function CompanionCanvas(
  props: CompanionCanvasProps,
): React.JSX.Element {
  const canvas = React.useRef<HTMLCanvasElement>(null),
    host = React.useRef<HTMLDivElement>(null);
  const latest = React.useRef(props);
  latest.current = props;
  const loop = React.useRef<FrameLoop | null>(null),
    poke = React.useRef<() => void>(() => {}),
    sync = React.useRef<() => void>(() => {});
  const pointer = React.useRef<Point | null>(null);
  React.useEffect(() => {
    const el = canvas.current!,
      container = host.current!;
    let context: CanvasRenderingContext2D | null = null;
    let disposed = false,
      visible = true,
      intersects = true;
    let viewport = { width: 0, height: 0, dpr: 1 };
    const motion = createMotion(viewport);
    let modelReady = false;
    let readinessReported = false;
    const reduced = matchMedia("(prefers-reduced-motion: reduce)");
    let dprQuery: MediaQueryList | null = null;
    const fail = () => {
      if (!disposed) {
        loop.current?.dispose();
        latest.current.onFailure("AYLUL canvas is unavailable.");
      }
    };
    try {
      context = el.getContext("2d");
    } catch {
      fail();
      return;
    }
    if (!context) {
      fail();
      return;
    }
    const frameLoop = createFrameLoop(
      {
        request: (callback) => requestAnimationFrame(callback),
        cancel: (id) => cancelAnimationFrame(id),
      },
      (seconds) => {
        if (
          disposed ||
          latest.current.renderAllowed === false ||
          !visible ||
          !intersects ||
          !viewport.width ||
          !viewport.height
        )
          return;
        try {
          const input = {
            ...latest.current.input,
            viewport,
            pointer: pointer.current ?? latest.current.input.pointer,
          };
          if (seconds > 0) advanceMotion(motion, seconds, input);
          const frame = readFrame(motion, input);
          context!.setTransform(viewport.dpr, 0, 0, viewport.dpr, 0, 0);
          context!.clearRect(0, 0, viewport.width, viewport.height);
          renderScene(context!, frame, input.settings);
          latest.current.onFrame?.(frame);
        } catch {
          fail();
        }
      },
    );
    loop.current = frameLoop;
    const reconcile = () => {
      visible = document.visibilityState !== "hidden";
      const canDraw =
        latest.current.renderAllowed !== false && visible && intersects && viewport.width > 0 && viewport.height > 0;
      if (!canDraw) {
        frameLoop.stop();
        return;
      }
      if (latest.current.paused || reduced.matches) {
        frameLoop.stop();
        frameLoop.invalidate();
      } else frameLoop.start();
    };
    sync.current = reconcile;
    const resize = () => {
      const bounds = container.getBoundingClientRect();
      viewport = {
        width: bounds.width,
        height: bounds.height,
        dpr: window.devicePixelRatio || 1,
      };
      const size = backingSize(viewport);
      if (el.width !== size.width) el.width = size.width;
      if (el.height !== size.height) el.height = size.height;
      if (!modelReady && viewport.width && viewport.height) {
        const fresh = createMotion(viewport);
        Object.assign(motion, fresh);
        modelReady = true;
      }
      pointer.current = null;
      if (!readinessReported && viewport.width > 0 && viewport.height > 0) {
        readinessReported = true;
        latest.current.onReady?.();
      }
      reconcile();
    };
    const dprChanged = () => {
      dprQuery?.removeEventListener("change", dprChanged);
      dprQuery = matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
      dprQuery.addEventListener("change", dprChanged);
      resize();
    };
    const ro = new ResizeObserver(resize);
    ro.observe(container);
    const io = new IntersectionObserver((entries) => {
      intersects = entries[0]?.isIntersecting ?? true;
      reconcile();
    });
    io.observe(container);
    document.addEventListener("visibilitychange", reconcile);
    reduced.addEventListener("change", reconcile);
    window.addEventListener("resize", resize);
    dprChanged();
    poke.current = () => {
      if (!latest.current.paused && !reduced.matches) pokeMotion(motion);
      latest.current.onPoke?.();
      reconcile();
    };
    return () => {
      disposed = true;
      frameLoop.dispose();
      ro.disconnect();
      io.disconnect();
      dprQuery?.removeEventListener("change", dprChanged);
      document.removeEventListener("visibilitychange", reconcile);
      reduced.removeEventListener("change", reconcile);
      window.removeEventListener("resize", resize);
      poke.current = () => {};
      sync.current = () => {};
      loop.current = null;
    };
  }, []);
  React.useEffect(() => {
    sync.current();
  }, [props.input, props.paused, props.renderAllowed]);
  return (
    <div
      className="aylul-canvas-region"
      ref={host}
      onPointerMove={(event) => {
        const bounds = event.currentTarget.getBoundingClientRect();
        pointer.current = localPointer(
          { x: event.clientX, y: event.clientY },
          bounds,
        );
      }}
      onPointerLeave={() => {
        pointer.current = null;
      }}
    >
      <button
        className="aylul-poke"
        type="button"
        aria-label={props.actionLabel ?? "Poke AYLUL"}
        onClick={() => poke.current()}
      >
        <canvas ref={canvas} aria-hidden="true" />
      </button>
    </div>
  );
}
