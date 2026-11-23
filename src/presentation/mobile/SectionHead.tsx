import { memo, type ReactNode } from "react";

/**
 * Numbered section band (Figma Kaze/Section Header): mono red kicker
 * ("02 // SUBTITLE"), black title and a 2px rule underneath.
 */
export const SectionHead = memo(function SectionHead({
  kicker, title, actions, compact = false, flush = false,
}: {
  kicker: string;
  title: string;
  actions?: ReactNode;
  compact?: boolean;
  flush?: boolean;
}) {
  const variant = `${compact ? " m-sechead--compact" : ""}${flush ? " m-sechead--flush" : ""}`;
  return (
    <header className={`m-sechead${variant}`}>
      <div className="m-sechead__text">
        <span className="m-kicker">{kicker}</span>
        <h2 className="m-sechead__title">{title}</h2>
      </div>
      {actions ? <div className="m-sechead__actions">{actions}</div> : null}
    </header>
  );
});
