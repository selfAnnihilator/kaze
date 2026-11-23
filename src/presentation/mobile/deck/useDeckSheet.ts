import { useLayoutEffect, useState } from "react";

const MOTION = "transform var(--duration-moderate) var(--ease-emphasized), opacity var(--duration-moderate) var(--ease-emphasized)";
/** Unmount a collapsing Deck even if its transition never reports the end. */
const COLLAPSE_FALLBACK_MS = 600;
const OPEN = Number.POSITIVE_INFINITY;
const CLOSED = 0;

/** The collapsed Deck's top edge: the mini player's top, else the bottom of the screen. */
export function collapsedY(): number {
  const mini = document.querySelector(".m-minideck");
  return mini ? mini.getBoundingClientRect().top : window.innerHeight;
}

/** Puts the Deck `travel` px up from the mini player; it turns solid over the first 12% of the way. */
function place(el: HTMLElement, travel: number, animate: boolean) {
  const distance = collapsedY();
  el.style.transition = animate ? MOTION : "none";
  el.style.transform = `translate3d(0, ${Math.max(0, distance - travel)}px, 0)`;
  el.style.opacity = String(Math.min(1, travel / (distance * 0.12)));
}

/** What a drag tells the sheet: it started, moved to `travel` px open, or ended open or not. */
export interface SheetDrag {
  start: () => void;
  move: (travel: number) => void;
  end: (open: boolean) => void;
}

/** Runs `then` once `el` finishes moving, or after a fallback if no transition ends. */
function afterMove(el: HTMLElement, then: () => void) {
  let timer = 0;
  const done = () => {
    el.removeEventListener("transitionend", onEnd);
    window.clearTimeout(timer);
    then();
  };
  const onEnd = (e: TransitionEvent) => { if (e.target === el && e.propertyName === "transform") done(); };
  el.addEventListener("transitionend", onEnd);
  timer = window.setTimeout(done, COLLAPSE_FALLBACK_MS);
}

type Actions = { open: () => void; close: () => void };

/** The Deck's motion, made once per shell: where it is wanted, and how it gets there. */
class DeckSheet implements SheetDrag {
  readonly ref: { current: HTMLElement | null } = { current: null };
  actions: Actions = { open: () => {}, close: () => {} };
  private deckOpen = false;
  private dragging = false;
  private want = { travel: CLOSED, animate: true };

  constructor(private readonly setMounted: (mounted: boolean) => void) {}

  private go(travel: number, animate: boolean) {
    this.want = { travel, animate };
    if (this.ref.current) place(this.ref.current, travel, animate);
    else this.setMounted(true);
  }

  private collapse() {
    const el = this.ref.current;
    if (!el) return;
    this.go(CLOSED, true);
    afterMove(el, () => {
      if (!this.deckOpen && !this.dragging && this.want.travel === CLOSED) this.setMounted(false);
    });
  }

  /** Just mounted: start folded into the mini player, then move to where it is wanted. */
  enter() {
    const el = this.ref.current;
    if (!el) return;
    place(el, CLOSED, false);
    void el.offsetHeight; // commit the start position so the move animates
    place(el, this.want.travel, this.want.animate);
  }

  /** `deckOpen` changed (a tap, ⌄, Android Back): follow it unless a finger holds the Deck. */
  follow(open: boolean) {
    this.deckOpen = open;
    if (this.dragging) return;
    if (open) this.go(OPEN, true);
    else this.collapse();
  }

  start = () => { this.dragging = true; };

  move = (travel: number) => this.go(Math.max(0, travel), false);

  /** A released drag: settle open (pushing the Back layer), or close the usual way. */
  end = (open: boolean) => {
    this.dragging = false;
    if (open) {
      this.go(OPEN, true);
      if (!this.deckOpen) this.actions.open();
    } else if (this.deckOpen) this.actions.close();
    else this.collapse();
  };
}

/**
 * The Deck as a sheet that grows out of the mini player and folds back into
 * it. `deckOpen` (tied to Android Back) says where it should be; drags move
 * it with the finger. Positions are written to the element, never to state.
 */
export function useDeckSheet(deckOpen: boolean, actions: Actions) {
  const [mounted, setMounted] = useState(deckOpen);
  const [motion] = useState(() => new DeckSheet(setMounted));
  motion.actions = actions;
  useLayoutEffect(() => { if (mounted) motion.enter(); }, [mounted, motion]);
  useLayoutEffect(() => motion.follow(deckOpen), [deckOpen, motion]);
  return { ref: motion.ref, mounted, drag: motion as SheetDrag };
}
