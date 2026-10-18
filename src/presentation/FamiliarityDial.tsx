import { memo, useEffect, useRef, useState, type CSSProperties } from "react";

const quantize = (value: number) =>
  Math.round(Math.max(0, Math.min(1, value)) * 20) / 20;
const description = (value: number) =>
  value <= 0.25
    ? "Rediscover: deep cuts and forgotten library gems"
    : value >= 0.75
      ? "Familiar: proven favourites and high affinity"
      : "Balanced blend: familiar favourites and rediscovery";

/** Dragging previews in this leaf. Only release or a keyboard step commits to Flow. */
export const FamiliarityDial = memo(function FamiliarityDial({
  value,
  onChange,
}: {
  value: number;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState(() => quantize(value));
  const drag = useRef<{
    x: number;
    y: number;
    angle: number;
    value: number;
  } | null>(null);
  const latest = useRef(draft);
  const controlled = useRef(value);
  controlled.current = value;
  useEffect(() => {
    if (!drag.current) {
      const next = quantize(value);
      latest.current = next;
      setDraft(next);
    }
  }, [value]);
  const preview = (next: number) => {
    const rounded = quantize(next);
    latest.current = rounded;
    setDraft(rounded);
  };
  const commit = (next: number) => {
    preview(next);
    onChange(quantize(next));
  };
  const cancel = () => {
    drag.current = null;
    preview(controlled.current);
  };
  return (
    <div className="familiarity-control">
      <span className="familiarity-label">Familiarity</span>
      <div
        className="familiarity-dial"
        role="slider"
        tabIndex={0}
        aria-label="Flow Steering"
        aria-valuemin={0}
        aria-valuemax={1}
        aria-valuenow={draft}
        aria-valuetext={`${Math.round(draft * 100)}% familiar. ${description(draft)}`}
        title={`${description(draft)}. Drag to turn, or use the arrow keys.`}
        style={{ "--dial-angle": `${-135 + draft * 270}deg` } as CSSProperties}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.focus({ preventScroll: true });
          const rect = event.currentTarget.getBoundingClientRect();
          const x = rect.x + rect.width / 2,
            y = rect.y + rect.height / 2;
          const angle =
            (Math.atan2(event.clientX - x, y - event.clientY) * 180) / Math.PI;
          const next =
            Math.abs(angle) > 135
              ? latest.current < 0.5
                ? 0
                : 1
              : (angle + 135) / 270;
          drag.current = { x, y, angle, value: next };
          preview(next);
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerMove={(event) => {
          const gesture = drag.current;
          if (
            !gesture ||
            !event.currentTarget.hasPointerCapture(event.pointerId)
          )
            return;
          const angle =
            (Math.atan2(event.clientX - gesture.x, gesture.y - event.clientY) *
              180) /
            Math.PI;
          const delta = ((angle - gesture.angle + 540) % 360) - 180;
          gesture.angle = angle;
          gesture.value = Math.max(0, Math.min(1, gesture.value + delta / 270));
          preview(gesture.value);
        }}
        onPointerUp={(event) => {
          if (!drag.current) return;
          const next = latest.current;
          drag.current = null;
          event.currentTarget.releasePointerCapture(event.pointerId);
          commit(next);
        }}
        onPointerCancel={cancel}
        onLostPointerCapture={() => {
          if (drag.current) cancel();
        }}
        onKeyDown={(event) => {
          if (event.altKey || event.ctrlKey || event.metaKey) return;
          const next =
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? 1
                : ["ArrowRight", "ArrowUp"].includes(event.key)
                  ? latest.current + 0.05
                  : ["ArrowLeft", "ArrowDown"].includes(event.key)
                    ? latest.current - 0.05
                    : event.key === "PageUp"
                      ? latest.current + 0.1
                      : event.key === "PageDown"
                        ? latest.current - 0.1
                        : null;
          if (next !== null) {
            event.preventDefault();
            event.stopPropagation();
            commit(next);
          } else if (event.key === " " || event.key === "Enter") {
            event.preventDefault();
            event.stopPropagation();
          }
        }}
      >
        <svg viewBox="0 0 120 120" aria-hidden="true">
          <path
            className="dial-track"
            d="M 23.23 96.77 A 52 52 0 1 1 96.77 96.77"
            pathLength="100"
          />
          <path
            className="dial-fill"
            d="M 23.23 96.77 A 52 52 0 1 1 96.77 96.77"
            pathLength="100"
            strokeDasharray={`${draft * 100} 100`}
          />
        </svg>
        <span className="dial-knob" aria-hidden="true">
          <span />
        </span>
        <span className="dial-value" aria-hidden="true">
          {Math.round(draft * 100)}
          <small>%</small>
        </span>
      </div>
      <div className="dial-endpoints" aria-hidden="true">
        <span>Rediscover</span>
        <span>Familiar</span>
      </div>
      <span className="spatial-sr-only">
        Changes guide the next generated batch.
      </span>
    </div>
  );
});
