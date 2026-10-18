import { useState, type CSSProperties, type ReactNode } from "react";

/** Table semantics plus viewport-bounded rows. Headers remain part of the table. */
export function WindowedTable<T>({
  items,
  renderItem,
  rowHeight = 66,
  children,
  className,
  style,
}: {
  items: readonly T[];
  renderItem: (item: T, index: number) => ReactNode;
  rowHeight?: number;
  children?: ReactNode;
  className?: string;
  style?: CSSProperties;
}) {
  const [row, setRow] = useState(0);
  const start = Math.max(0, row - 3);
  const end = Math.min(items.length, row + Math.ceil(540 / rowHeight) + 4);
  return (
    <div
      className="windowed-table"
      style={{ maxHeight: 540, overflow: "auto" }}
      onScroll={(e) => {
        const next = Math.floor(e.currentTarget.scrollTop / rowHeight);
        if (next !== row)
          setRow((current) => (current === next ? current : next));
      }}
    >
      <table className={className} style={style}>
        {children}
        <tbody>
          {start > 0 && (
            <tr aria-hidden="true">
              <td
                colSpan={99}
                style={{ height: start * rowHeight, padding: 0, border: 0 }}
              />
            </tr>
          )}
          {items.slice(start, end).map((item, index) => (
            <FixedRow key={start + index} height={rowHeight}>
              {renderItem(item, start + index)}
            </FixedRow>
          ))}
          {end < items.length && (
            <tr aria-hidden="true">
              <td
                colSpan={99}
                style={{
                  height: (items.length - end) * rowHeight,
                  padding: 0,
                  border: 0,
                }}
              />
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

// cloneElement keeps valid <tr>/<td> semantics while fixing the virtual pitch.
import { cloneElement, isValidElement, type ReactElement } from "react";
function FixedRow({
  children,
  height,
}: {
  children: ReactNode;
  height: number;
}) {
  if (!isValidElement(children)) return children;
  const row = children as ReactElement<{ style?: CSSProperties }>;
  return cloneElement(row, { style: { ...row.props.style, height } });
}
