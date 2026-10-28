import { useEffect } from "react";

/** One delegated mouse gesture for all shelves, including windowed/replaced rows.
 * Movement stays in the DOM; React only sees existing virtualization boundaries. */
export function useHorizontalDrag() {
  useEffect(() => {
    let drag: {
      el: HTMLElement; id: number; x: number; y: number; left: number;
      moved: boolean; behavior: string; snap: string;
    } | null = null;
    let suppressClick: HTMLElement | null = null;
    const finish = () => {
      const state = drag;
      drag = null;
      if (!state) return;
      state.el.style.scrollBehavior = state.behavior;
      state.el.style.scrollSnapType = state.snap;
      state.el.removeAttribute("data-dragging");
      if (state.el.hasPointerCapture(state.id)) state.el.releasePointerCapture(state.id);
    };
    const down = (event: PointerEvent) => {
      finish();
      suppressClick = null;
      if (event.pointerType !== "mouse" || event.button !== 0 || !event.isPrimary ||
          event.ctrlKey || event.altKey || event.metaKey || event.shiftKey ||
          !(event.target instanceof Element)) return;
      const el = event.target.closest<HTMLElement>(".kaze-interface .horizontal-scroll-row");
      if (!el || el.scrollWidth <= el.clientWidth || event.target.closest(
        "input, textarea, select, [contenteditable]:not([contenteditable='false']), [role='slider']",
      )) return;
      drag = { el, id: event.pointerId, x: event.clientX, y: event.clientY,
        left: el.scrollLeft, moved: false, behavior: el.style.scrollBehavior, snap: el.style.scrollSnapType };
    };
    const move = (event: PointerEvent) => {
      const state = drag;
      if (!state || event.pointerId !== state.id) return;
      if (!(event.buttons & 1) || !state.el.isConnected) { finish(); return; }
      const distance = event.clientX - state.x;
      if (!state.moved) {
        if (Math.abs(event.clientY - state.y) > Math.max(6, Math.abs(distance))) { finish(); return; }
        if (Math.abs(distance) <= 6) return;
        state.moved = true;
        state.el.style.scrollBehavior = "auto";
        state.el.style.scrollSnapType = "none";
        state.el.setAttribute("data-dragging", "");
        state.el.setPointerCapture(event.pointerId);
        suppressClick = state.el;
      }
      event.preventDefault();
      state.el.scrollLeft = state.left - distance;
    };
    const end = (event: PointerEvent) => { if (drag?.id === event.pointerId) finish(); };
    const click = (event: MouseEvent) => {
      if (event.detail > 0 && suppressClick && event.target instanceof Node && suppressClick.contains(event.target)) {
        event.preventDefault();
        event.stopPropagation();
        suppressClick = null;
      }
    };
    const nativeDrag = (event: DragEvent) => { if (drag) event.preventDefault(); };
    document.addEventListener("pointerdown", down, true);
    document.addEventListener("pointermove", move, { capture: true, passive: false });
    document.addEventListener("pointerup", end, true);
    document.addEventListener("pointercancel", end, true);
    document.addEventListener("lostpointercapture", end, true);
    document.addEventListener("click", click, true);
    document.addEventListener("dragstart", nativeDrag, true);
    window.addEventListener("blur", finish);
    return () => {
      finish();
      document.removeEventListener("pointerdown", down, true);
      document.removeEventListener("pointermove", move, true);
      document.removeEventListener("pointerup", end, true);
      document.removeEventListener("pointercancel", end, true);
      document.removeEventListener("lostpointercapture", end, true);
      document.removeEventListener("click", click, true);
      document.removeEventListener("dragstart", nativeDrag, true);
      window.removeEventListener("blur", finish);
    };
  }, []);
}
