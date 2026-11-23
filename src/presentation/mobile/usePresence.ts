import { useEffect, useState } from "react";

/** How long a panel plays its closing animation; matches --duration-moderate in mobile-motion.css. */
export const LEAVE_MS = 250;

/**
 * Keeps a panel mounted for its closing animation: `mounted` stays true for
 * LEAVE_MS after `open` turns false, with `leaving` set meanwhile.
 */
export function usePresence(open: boolean) {
  const [mounted, setMounted] = useState(open);
  useEffect(() => {
    if (open) { setMounted(true); return; }
    const timer = window.setTimeout(() => setMounted(false), LEAVE_MS);
    return () => window.clearTimeout(timer);
  }, [open]);
  return { mounted: open || mounted, leaving: !open && mounted };
}
