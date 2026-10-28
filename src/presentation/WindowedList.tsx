import React from "react";
import { List, type RowComponentProps } from "react-window";

type Rows<T> = {
  items: readonly T[];
  renderItem: (item: T, index: number) => React.ReactNode;
};
function Row<T>({
  index,
  style,
  items,
  renderItem,
}: RowComponentProps<Rows<T>>) {
  return <div style={style}>{renderItem(items[index], index)}</div>;
}

/** One bounded DOM contract for collections and other potentially long lists. */
export function WindowedList<T>({
  items,
  renderItem,
  rowHeight = 72,
  label = "Tracks",
  fill = false,
}: Rows<T> & { rowHeight?: number; label?: string; fill?: boolean }) {
  return (
    <List
      aria-label={label}
      rowComponent={Row<T>}
      rowCount={items.length}
      rowHeight={rowHeight}
      rowProps={{ items, renderItem }}
      overscanCount={3}
      style={{
        height: fill ? "100%" : Math.min(560, items.length * rowHeight),
        flex: fill ? "1 1 0" : undefined,
        minHeight: fill ? 0 : Math.min(rowHeight, items.length * rowHeight),
        width: "100%",
      }}
    />
  );
}
