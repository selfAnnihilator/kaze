import type { ReactNode } from "react";

/**
 * Centered dialog (Figma G7/G8/G9): dimmed screen, a paper card with a red top
 * bar, 2px ink border and hard shadow; kicker, title, body, then the actions.
 */
export function MobileDialog({ kicker, title, children, actions, onDismiss }: {
  kicker: string; title: string; children?: ReactNode; actions: ReactNode; onDismiss?: () => void;
}) {
  return (
    <div className="m-dialog" role="presentation" onClick={(e) => { if (e.target === e.currentTarget) onDismiss?.(); }}>
      <section className="m-dialog__card" role="dialog" aria-modal="true" aria-label={title}>
        <div className="m-dialog__bar" />
        <div className="m-dialog__body">
          <span className="m-kicker">{kicker}</span>
          <h2 className="m-display m-dialog__title">{title}</h2>
          {children}
        </div>
        <div className="m-dialog__actions">{actions}</div>
      </section>
    </div>
  );
}
