import type { ReactNode } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";

type Direction = "left" | "right";

interface ShelfHeaderProps {
  icon: ReactNode;
  title: string;
  subtitle: string;
  onScroll: (direction: Direction) => void;
  /** Gap between the two arrow buttons. */
  arrowGap?: string;
  headingId?: string;
  showArrows?: boolean;
  /** Names the shelf in the arrows' title and aria-label ("Scroll For You left"). */
  arrowLabel?: string;
}

function ScrollArrow({ direction, label, onScroll }: { direction: Direction; label?: string; onScroll: (direction: Direction) => void }) {
  const text = label ? `Scroll ${label} ${direction}` : `Scroll ${direction}`;
  return (
    <button type="button" className="scroll-arrow-btn" onClick={() => onScroll(direction)} title={text} aria-label={label ? text : undefined}>
      {direction === "left" ? <ChevronLeft size={16} /> : <ChevronRight size={16} />}
    </button>
  );
}

/** Title, subtitle and scroll arrows above a horizontal Discover shelf. */
export function ShelfHeader({ icon, title, subtitle, onScroll, arrowGap = "6px", headingId, showArrows = true, arrowLabel }: ShelfHeaderProps) {
  return (
    <div
      className="discovery-shelf-header"
      style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "var(--space-12)" }}
    >
      <div>
        <h2
          id={headingId}
          style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", color: "var(--text-main)", display: "flex", alignItems: "center", gap: "var(--space-8)" }}
        >
          {icon}
          <span>{title}</span>
        </h2>
        <p style={{ fontSize: "var(--fs-13)", color: "var(--text-muted)", marginTop: "var(--space-2)" }}>{subtitle}</p>
      </div>
      {showArrows && (
        <div style={{ display: "flex", alignItems: "center", gap: arrowGap }}>
          <ScrollArrow direction="left" label={arrowLabel} onScroll={onScroll} />
          <ScrollArrow direction="right" label={arrowLabel} onScroll={onScroll} />
        </div>
      )}
    </div>
  );
}
