import { useEffect, useRef, type RefObject } from "react";
import { pullArmed, pullStarted } from "./pullMath";
import { PullView } from "./pullView";

/** The scrolling page the indicator sits in. */
export const scrollerOf = (el: HTMLElement) => el.closest<HTMLElement>(".m-screen__content, .m-page__body");

type Pull = { x: number; y: number; dy: number; on: boolean; top: number };

function pullGesture(view: PullView, scroller: HTMLElement, refresh: () => Promise<unknown>) {
  let g: Pull | null = null;
  return {
    start(e: TouchEvent) {
      const t = e.touches[0];
      g = { x: t.clientX, y: t.clientY, dy: 0, on: false, top: scroller.scrollTop };
    },
    move(e: TouchEvent) {
      if (!g) return;
      const t = e.touches[0];
      g.dy = t.clientY - g.y;
      if (!g.on) g.on = pullStarted(t.clientX - g.x, g.dy, g.top);
      if (!g.on) return;
      // Only an active pull takes over the touch; scrolling and taps stay native.
      if (e.cancelable) e.preventDefault();
      view.follow(g.dy);
    },
    end() {
      if (g?.on) {
        if (pullArmed(g.dy)) void view.spin(refresh());
        else view.reset();
      }
      g = null;
    },
  };
}

/**
 * Pull down at the top of the page to refresh (replaces the ⟳ buttons on the
 * phone). `indicator` is a PullIndicator inside the page's scroller.
 */
export function usePullToRefresh(indicator: RefObject<HTMLElement | null>, onRefresh: () => Promise<unknown> | void) {
  const latest = useRef(onRefresh);
  latest.current = onRefresh;
  useEffect(() => {
    const el = indicator.current;
    const scroller = el && scrollerOf(el);
    if (!el || !scroller) return;
    const g = pullGesture(new PullView(el), scroller, () => Promise.resolve(latest.current()));
    scroller.addEventListener("touchstart", g.start, { passive: true });
    scroller.addEventListener("touchmove", g.move, { passive: false });
    scroller.addEventListener("touchend", g.end, { passive: true });
    scroller.addEventListener("touchcancel", g.end, { passive: true });
    return () => {
      scroller.removeEventListener("touchstart", g.start);
      scroller.removeEventListener("touchmove", g.move);
      scroller.removeEventListener("touchend", g.end);
      scroller.removeEventListener("touchcancel", g.end);
    };
  }, [indicator]);
}
