import {
  memo,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
} from "react";

/** Measure only text/size changes; hover animation stays entirely in CSS. */
export const OverflowTitle = memo(function OverflowTitle({
  text,
  focusable = false,
}: {
  text: string;
  focusable?: boolean;
}) {
  const frame = useRef<HTMLSpanElement>(null);
  const label = useRef<HTMLSpanElement>(null);
  const [{ travel, duration }, setMetrics] = useState({
    travel: 0,
    duration: 4,
  });
  useLayoutEffect(() => {
    const measure = () => {
      if (frame.current && label.current) {
        const travel = Math.max(
          0,
          label.current.scrollWidth - frame.current.clientWidth,
        );
        const fontSize = parseFloat(getComputedStyle(frame.current).fontSize);
        const duration = Math.min(
          18,
          Math.max(4, travel / Math.max(48, fontSize * 3) + 1.5),
        );
        setMetrics((previous) =>
          previous.travel === travel && previous.duration === duration
            ? previous
            : { travel, duration },
        );
      }
    };
    measure();
    const observer = new ResizeObserver(measure);
    if (frame.current) observer.observe(frame.current);
    if (label.current) observer.observe(label.current);
    document.fonts.ready.then(measure);
    return () => observer.disconnect();
  }, [text]);
  return (
    <span
      ref={frame}
      className="overflow-title"
      data-overflow={travel > 1 ? "true" : undefined}
      tabIndex={focusable && travel > 1 ? 0 : undefined}
      title={text}
      style={
        {
          "--title-travel": `${-travel}px`,
          "--title-duration": `${duration}s`,
        } as CSSProperties
      }
    >
      <span ref={label} className="overflow-title-label">
        {text}
      </span>
      <span key={text} className="overflow-title-reveal" aria-hidden="true">
        {text}
      </span>
    </span>
  );
});
