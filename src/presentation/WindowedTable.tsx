import {
  Children,
  cloneElement,
  isValidElement,
  useState,
  type CSSProperties,
  type ReactElement,
  type ReactNode,
} from "react";

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
  const rows = items.slice(start, end).map((item, index) => renderItem(item, start + index));
  // Spacer cells must span the real column count: under `table-layout: fixed`
  // an oversized colSpan adds phantom columns that squeeze the visible ones.
  const columns = columnCount(rows[0]) || headerColumns(children) || 1;
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
                colSpan={columns}
                style={{ height: start * rowHeight, padding: 0, border: 0 }}
              />
            </tr>
          )}
          {rows.map((content, index) => (
            <FixedRow key={start + index} height={rowHeight}>
              {content}
            </FixedRow>
          ))}
          {end < items.length && (
            <tr aria-hidden="true">
              <td
                colSpan={columns}
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

function columnCount(node: ReactNode): number {
  if (!isValidElement(node)) return 0;
  const props = node.props as { children?: ReactNode };
  if (node.type === "tr") {
    return Children.toArray(props.children).reduce<number>((sum, cell) => {
      if (!isValidElement(cell)) return sum;
      const span = (cell.props as { colSpan?: number }).colSpan;
      return sum + (span && span > 0 ? span : 1);
    }, 0);
  }
  // Wrapper components (e.g. render-prop rows) cannot be inspected statically.
  return 0;
}

/** Fallback when rows are wrapped: count cells of the first header row. */
function headerColumns(children: ReactNode): number {
  for (const child of Children.toArray(children)) {
    if (isValidElement(child) && child.type === "thead") {
      const first = Children.toArray((child.props as { children?: ReactNode }).children)[0];
      return columnCount(first);
    }
  }
  return 0;
}
