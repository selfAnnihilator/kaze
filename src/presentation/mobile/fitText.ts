import { useLayoutEffect, useRef } from "react";

/**
 * Shrinks an element's font so its one line fits its box (never grows past
 * the CSS size). Runs when the text changes and when the box resizes.
 */
export function useFitText<T extends HTMLElement>(text: string) {
  const ref = useRef<T>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const fit = () => {
      el.style.fontSize = "";
      for (let pass = 0; pass < 2 && el.scrollWidth > el.clientWidth; pass++) {
        const size = parseFloat(getComputedStyle(el).fontSize);
        el.style.fontSize = `${Math.floor((size * el.clientWidth) / el.scrollWidth)}px`;
      }
    };
    fit();
    // Watch the box it sits in: our own font changes must not re-trigger a fit.
    const observer = new ResizeObserver(fit);
    observer.observe(el.parentElement ?? el);
    return () => observer.disconnect();
  }, [text]);
  return ref;
}
