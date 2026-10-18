import {
  useLayoutEffect,
  memo,
  useRef,
  useState,
  type ReactNode,
  type RefObject,
} from "react";

/** Native horizontal scrolling with a bounded number of mounted sleeves. */
export function WindowedStrip<T>({
  items,
  renderItem,
  scrollRef,
  onScroll,
  label,
  initialScrollLeft,
  initialScrollTop = 0,
}: {
  items: readonly T[];
  renderItem: (item: T, index: number) => ReactNode;
  scrollRef?: RefObject<HTMLDivElement | null>;
  onScroll?: () => void;
  label: string;
  initialScrollLeft?: number;
  initialScrollTop?: number;
}) {
  const local = useRef<HTMLDivElement>(null);
  const ref = scrollRef || local;
  const [viewport, setViewport] = useState({
    bucket: 0,
    width: 1000,
    sleeve: 212,
    gap: 24,
    measured: false,
  });
  const initialPosition = useRef({
    left: initialScrollLeft,
    top: initialScrollTop,
  });
  const restored = useRef(false);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const measure = () => {
      const css = getComputedStyle(el);
      const sleeve =
        parseFloat(css.getPropertyValue("--strip-item-width")) || 212;
      const gap = parseFloat(css.getPropertyValue("--strip-item-gap")) || 24;
      const next = {
        bucket: Math.floor(el.scrollLeft / (sleeve + gap)),
        width: el.clientWidth,
        sleeve,
        gap,
        measured: true,
      };
      setViewport((current) =>
        current.measured &&
        current.bucket === next.bucket &&
        current.width === next.width &&
        current.sleeve === sleeve &&
        current.gap === gap
          ? current
          : next,
      );
    };
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    measure();
    return () => observer.disconnect();
  }, [ref]);
  useLayoutEffect(() => {
    if (!viewport.measured || !items.length || restored.current) return;
    restored.current = true;
    // The initial estimated sleeve width can clamp a saved offset too early.
    // Restore only after CSS measurements have produced the real strip width.
    if (initialPosition.current.left !== undefined) {
      ref.current?.scrollTo({
        ...initialPosition.current,
        behavior: "instant",
      });
      const bucket = Math.floor(
        (ref.current?.scrollLeft || 0) / (viewport.sleeve + viewport.gap),
      );
      setViewport((current) =>
        current.bucket === bucket ? current : { ...current, bucket },
      );
    }
  }, [viewport.measured, viewport.sleeve, viewport.gap, items.length, ref]);
  const pitch = viewport.sleeve + viewport.gap;
  const start = Math.max(0, viewport.bucket - 2);
  const end = Math.min(
    items.length,
    viewport.bucket + Math.ceil(viewport.width / pitch) + 3,
  );
  return (
    <div
      className="horizontal-scroll-row windowed-strip"
      ref={ref}
      tabIndex={0}
      role="region"
      aria-label={label}
      onScroll={() => {
        if (ref.current) {
          const bucket = Math.floor(ref.current.scrollLeft / pitch);
          if (bucket !== viewport.bucket)
            setViewport((current) =>
              current.bucket === bucket ? current : { ...current, bucket },
            );
        }
        onScroll?.();
      }}
    >
      <div
        className="windowed-strip-inner"
        style={{
          width: Math.max(0, items.length * pitch - viewport.gap),
          minWidth: "100%",
        }}
      >
        {items.slice(start, end).map((item, i) => (
          <MemoSleeve
            key={start + i}
            item={item}
            index={start + i}
            left={(start + i) * pitch}
            width={viewport.sleeve}
            renderItem={renderItem}
          />
        ))}
      </div>
    </div>
  );
}

function Sleeve<T>({
  item,
  index,
  left,
  width,
  renderItem,
}: {
  item: T;
  index: number;
  left: number;
  width: number;
  renderItem: (item: T, index: number) => ReactNode;
}) {
  return (
    <div className="windowed-sleeve" style={{ left, width }}>
      {renderItem(item, index)}
    </div>
  );
}
const MemoSleeve = memo(Sleeve) as typeof Sleeve;
