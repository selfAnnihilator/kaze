import { useCallback, useEffect, useRef, useState } from "react";
import { navigationShortcut, replaceDestination, stepHistory, viewDestination, visitDestination } from "../navigationHistory";
import type { NavigationDestination, NavigationHistory } from "../navigationHistory";

export function useNavigationHistory() {
  const [history, setHistory] = useState<NavigationHistory>(() => ({ entries: [viewDestination("home")], index: 0 }));
  const historyRef = useRef(history);
  const commit = useCallback((next: NavigationHistory) => {
    if (next === historyRef.current) return false;
    historyRef.current = next;
    setHistory(next);
    return true;
  }, []);
  const navigate = useCallback((destination: NavigationDestination) => commit(visitDestination(historyRef.current, destination)), [commit]);
  const updateDestination = useCallback((update: (current: NavigationDestination) => NavigationDestination) => {
    const h = historyRef.current;
    const current = h.entries[h.index];
    const next = update(current);
    if (next !== current) commit(replaceDestination(h, next));
  }, [commit]);
  const back = useCallback(() => commit(stepHistory(historyRef.current, -1)), [commit]);
  const forward = useCallback(() => commit(stepHistory(historyRef.current, 1)), [commit]);
  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      const editable = event.composedPath().some(target => target instanceof Element && !!target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"]), [role="textbox"], [role="slider"], [role="combobox"], [role="dialog"], [aria-modal="true"]'));
      // Dialogs may have focus outside their contents; leave modal shortcuts alone.
      if (document.querySelector('.modal-overlay, [aria-modal="true"]')) return;
      const direction = navigationShortcut(event, editable);
      if (direction != null && (direction === -1 ? back() : forward())) event.preventDefault();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [back, forward]);
  return { destination: history.entries[history.index], navigate, updateDestination, back, forward, canGoBack: history.index > 0, canGoForward: history.index < history.entries.length - 1 };
}
