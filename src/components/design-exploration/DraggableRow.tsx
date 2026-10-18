import React, { useRef, useState, useCallback } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";

interface DraggableRowProps {
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
  showNavButtons?: boolean;
}

export const DraggableRow: React.FC<DraggableRowProps> = ({
  children,
  className = "",
  style = {},
  showNavButtons = true,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const isDraggingRef = useRef(false);
  const startXRef = useRef(0);
  const scrollLeftRef = useRef(0);
  const hasMovedRef = useRef(false);
  const [isGrabbing, setIsGrabbing] = useState(false);

  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    // Only handle primary button (left mouse click or touch)
    if (e.button !== 0) return;
    const container = containerRef.current;
    if (!container) return;

    isDraggingRef.current = true;
    hasMovedRef.current = false;
    startXRef.current = e.pageX - container.offsetLeft;
    scrollLeftRef.current = container.scrollLeft;
    setIsGrabbing(true);

    container.setPointerCapture(e.pointerId);
  }, []);

  const handlePointerMove = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (!isDraggingRef.current) return;
    const container = containerRef.current;
    if (!container) return;

    const x = e.pageX - container.offsetLeft;
    const walk = (x - startXRef.current) * 1.35; // Slight momentum multiplier for satisfying drag
    if (Math.abs(walk) > 4) {
      hasMovedRef.current = true;
    }
    container.scrollLeft = scrollLeftRef.current - walk;
  }, []);

  const handlePointerUp = useCallback((e: React.PointerEvent<HTMLDivElement>) => {
    if (!isDraggingRef.current) return;
    isDraggingRef.current = false;
    setIsGrabbing(false);
    const container = containerRef.current;
    if (container && container.hasPointerCapture(e.pointerId)) {
      container.releasePointerCapture(e.pointerId);
    }
  }, []);

  const handleScroll = (direction: "left" | "right") => {
    if (!containerRef.current) return;
    const scrollAmount = containerRef.current.clientWidth * 0.75;
    containerRef.current.scrollBy({
      left: direction === "left" ? -scrollAmount : scrollAmount,
      behavior: "smooth",
    });
  };

  return (
    <div className={`draggable-row-wrapper ${className}`} style={{ position: "relative", ...style }}>
      {showNavButtons && (
        <>
          <button
            type="button"
            className="draggable-nav-btn prev"
            onClick={() => handleScroll("left")}
            aria-label="Scroll left"
          >
            <ChevronLeft size={18} />
          </button>
          <button
            type="button"
            className="draggable-nav-btn next"
            onClick={() => handleScroll("right")}
            aria-label="Scroll right"
          >
            <ChevronRight size={18} />
          </button>
        </>
      )}

      <div
        ref={containerRef}
        className={`draggable-row-track ${isGrabbing ? "is-grabbing" : ""}`}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        onClickCapture={(e) => {
          // If the user dragged more than 4px, suppress the click on cards
          if (hasMovedRef.current) {
            e.stopPropagation();
            e.preventDefault();
          }
        }}
        style={{
          display: "flex",
          overflowX: "auto",
          overflowY: "hidden",
          overscrollBehaviorX: "contain",
          scrollBehavior: isGrabbing ? "auto" : "smooth",
          cursor: isGrabbing ? "grabbing" : "grab",
          userSelect: "none",
          WebkitUserSelect: "none",
          scrollbarWidth: "none", // standard Firefox
          msOverflowStyle: "none", // IE
        }}
      >
        {children}
      </div>
    </div>
  );
};
