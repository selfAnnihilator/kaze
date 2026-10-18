import { memo, useEffect, useRef, useState, type CSSProperties } from "react";

const quantize = (value: number) =>
  Math.round(Math.max(0, Math.min(1, value)) * 20) / 20;

/** Brutalist steering previews locally and commits through the existing Flow callback. */
export const FamiliarityBar = memo(function FamiliarityBar({
  value,
  onChange,
}: {
  value: number;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState(() => quantize(value));
  const dragging = useRef(false);
  const latest = useRef(draft);
  const controlled = useRef(value);
  controlled.current = value;
  useEffect(() => {
    if (!dragging.current) {
      latest.current = quantize(value);
      setDraft(latest.current);
    }
  }, [value]);
  const preview = (next: number) => {
    latest.current = quantize(next);
    setDraft(latest.current);
  };
  const commit = (next: number) => {
    preview(next);
    onChange(quantize(next));
  };
  const cancel = () => {
    dragging.current = false;
    preview(controlled.current);
  };
  const description =
    draft <= 0.25
      ? "Rediscover: deep cuts and forgotten library gems"
      : draft >= 0.75
        ? "Familiar: proven favourites and high affinity"
        : "Balanced blend: familiar favourites and rediscovery";
  return (
    <div className="press-familiarity">
      <div className="press-familiarity-heading">
        <label htmlFor="press-flow-steering">Familiarity</label>
        <output htmlFor="press-flow-steering">
          {Math.round(draft * 100)}%
        </output>
      </div>
      <div
        className="press-familiarity-track"
        style={{ "--steering-fill": draft } as CSSProperties}
      >
        <span className="press-familiarity-fill" aria-hidden="true" />
        <input
          id="press-flow-steering"
          type="range"
          aria-label="Flow Steering"
          aria-valuetext={`${Math.round(draft * 100)}% familiar. ${description}`}
          title={description}
          min={0}
          max={1}
          step={0.05}
          value={draft}
          onPointerDown={event => {
            if (event.button !== 0) return;
            dragging.current = true;
            event.currentTarget.setPointerCapture(event.pointerId);
          }}
          onChange={(event) => {
            const next = Number(event.target.value);
            if (dragging.current) preview(next);
            else commit(next);
          }}
          onPointerUp={event => {
            if (!dragging.current) return;
            dragging.current = false;
            commit(latest.current);
            if (event.currentTarget.hasPointerCapture(event.pointerId))
              event.currentTarget.releasePointerCapture(event.pointerId);
          }}
          onPointerCancel={cancel}
          onLostPointerCapture={() => {
            if (dragging.current) cancel();
          }}
          onBlur={() => {
            if (dragging.current) cancel();
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
        />
      </div>
      <div className="press-familiarity-endpoints" aria-hidden="true">
        <span>Rediscover</span>
        <span>Familiar</span>
      </div>
    </div>
  );
});
