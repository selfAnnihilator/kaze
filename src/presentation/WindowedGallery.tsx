import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { WindowedList } from "./WindowedList";

export function WindowedGallery<T>({
  items,
  renderItem,
  label,
  layout,
}: {
  items: readonly T[];
  renderItem: (item: T, index: number) => ReactNode;
  label: string;
  layout?: "shelf" | "rows" | "index";
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [{ columns, rowHeight }, setMetrics] = useState({
    columns:
      layout === "shelf" ? 5 : layout === "rows" || layout === "index" ? 1 : 3,
    rowHeight: layout === "index" ? 148 : layout === "rows" ? 184 : 390,
  });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(() => {
      const columns =
        layout === "rows" || layout === "index"
          ? 1
          : layout === "shelf"
            ? 5
            : Math.max(1, Math.floor(el.clientWidth / 230));
      const unit = Math.max(50, (el.clientWidth - 150) / 6);
      setMetrics({
        columns,
        rowHeight:
          layout === "index"
            ? 148
            : layout === "rows"
              ? 184
              : layout === "shelf"
                ? Math.ceil(unit * 4 + 346)
                : Math.ceil((el.clientWidth - (columns - 1) * 24) / columns) +
                  155,
      });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [layout]);
  const rows = useMemo(
    () =>
      Array.from({ length: Math.ceil(items.length / columns) }, (_, index) =>
        items.slice(index * columns, (index + 1) * columns),
      ),
    [items, columns],
  );
  return (
    <div className="windowed-gallery" ref={ref}>
      <WindowedList
        items={rows}
        rowHeight={rowHeight}
        label={label}
        renderItem={(row, rowIndex) => (
          <div
            className={`gallery-row${layout === "shelf" ? " spatial-album-group" : layout === "rows" ? " spatial-playlist-row" : layout === "index" ? " press-playlist-row" : ""}`}
            style={{
              gridTemplateColumns: `repeat(${layout === "shelf" ? 6 : columns}, minmax(0, 1fr))`,
            }}
          >
            {row.map((item, index) => (
              <div className="gallery-item" key={rowIndex * columns + index}>
                {renderItem(item, rowIndex * columns + index)}
              </div>
            ))}
          </div>
        )}
      />
    </div>
  );
}
