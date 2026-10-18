import { useEffect, type RefObject } from "react";

const animated =
  ".spin-animation, .animate-spin, .spinning, .spinner, .discovery-eq-bar, .eq-bar";

/** Browser visibility is reliable for the native WebKitGTK tray hide/show path.
 * No React state, visual polling, scroll listener or per-frame measurement. */
export function useVisualActivity(main: RefObject<HTMLElement | null>) {
  useEffect(() => {
    const app = main.current?.closest(".kaze-interface");
    if (!app) return;
    const visibility = () => {
      document.documentElement.toggleAttribute(
        "data-visuals-paused",
        document.hidden,
      );
    };
    visibility();
    document.addEventListener("visibilitychange", visibility);
    const targets = new Set<Element>();
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries)
        entry.target.toggleAttribute(
          "data-animation-offscreen",
          !entry.isIntersecting,
        );
    });
    const track = (element: Element) => {
      if (element.matches(animated) && !targets.has(element)) {
        targets.add(element);
        observer.observe(element);
      }
    };
    const add = (element: Element) => {
      track(element);
      element.querySelectorAll(animated).forEach(track);
    };
    add(app);
    const changes = new MutationObserver((records) => {
      for (const record of records) {
        if (record.type === "attributes") track(record.target as Element);
        else
          record.addedNodes.forEach((node) => {
            if (node instanceof Element) add(node);
          });
      }
      for (const target of targets) {
        if (!app.contains(target) || !target.matches(animated)) {
          observer.unobserve(target);
          targets.delete(target);
          target.removeAttribute("data-animation-offscreen");
        }
      }
    });
    changes.observe(app, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => {
      changes.disconnect();
      observer.disconnect();
      targets.forEach((target) =>
        target.removeAttribute("data-animation-offscreen"),
      );
      document.removeEventListener("visibilitychange", visibility);
      document.documentElement.removeAttribute("data-visuals-paused");
    };
  }, [main]);
}
