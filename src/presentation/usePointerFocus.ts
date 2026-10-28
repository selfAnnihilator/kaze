import { useEffect } from "react";

/** Keep a pointer-focused control from acquiring a ring on an unrelated key.
 * Tab restores native keyboard focus styling. No React state or blur is needed. */
export function usePointerFocus() {
  useEffect(() => {
    let pointerControl: Element | null = null;
    const clear = () => {
      pointerControl?.removeAttribute("data-pointer-focus");
      pointerControl = null;
    };
    const pointerDown = (event: PointerEvent) => {
      clear();
      const control = event.target instanceof Element
        ? event.target.closest("button, a[href], [role='button'], [role='tab'], [role='menuitem']")
        : null;
      if (control?.closest(".kaze-interface")) {
        pointerControl = control;
        control.setAttribute("data-pointer-focus", "");
      }
    };
    const keyDown = (event: KeyboardEvent) => {
      if (event.key === "Tab" && !event.ctrlKey && !event.altKey && !event.metaKey) clear();
    };
    const focusOut = (event: FocusEvent) => {
      if (event.target === pointerControl) clear();
    };
    document.addEventListener("pointerdown", pointerDown, true);
    document.addEventListener("keydown", keyDown, true);
    document.addEventListener("focusout", focusOut, true);
    return () => {
      document.removeEventListener("pointerdown", pointerDown, true);
      document.removeEventListener("keydown", keyDown, true);
      document.removeEventListener("focusout", focusOut, true);
      clear();
    };
  }, []);
}
