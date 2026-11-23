import { forwardRef } from "react";
import { RotateCw } from "lucide-react";

/** The square ⟳ tile that slides down from the top while pulling to refresh. */
export const PullIndicator = forwardRef<HTMLDivElement>(function PullIndicator(_, ref) {
  return (
    <div ref={ref} className="m-pull" aria-hidden="true">
      <span className="m-pull__tile"><RotateCw size={18} strokeWidth={2} className="m-pull__icon" /></span>
    </div>
  );
});
