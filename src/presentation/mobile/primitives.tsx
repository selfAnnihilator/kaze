import { memo, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode } from "react";

type ButtonVariant = "primary" | "ghost";

/** Rectangular, high-contrast action button. */
export const MobileButton = memo(function MobileButton({
  variant = "primary", className = "", ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: ButtonVariant }) {
  return <button type="button" {...props} className={`m-button m-button--${variant} ${className}`} />;
});

/** Square touch target holding one icon. */
export const MobileIconButton = memo(function MobileIconButton({
  label, active = false, className = "", ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; active?: boolean }) {
  return (
    <button type="button" aria-label={label} aria-pressed={active || undefined} {...props}
      className={`m-icon-button ${active ? "is-active" : ""} ${className}`} />
  );
});

/** Selectable filter chip. */
export const MobileChip = memo(function MobileChip({
  selected, onSelect, children,
}: { selected: boolean; onSelect: () => void; children: ReactNode }) {
  return (
    <button type="button" role="tab" aria-selected={selected} onClick={onSelect}
      className={`m-chip ${selected ? "is-selected" : ""}`}>
      {children}
    </button>
  );
});

export function MobileChipRow({ label, children }: { label: string; children: ReactNode }) {
  return <div className="m-chip-row" role="tablist" aria-label={label}>{children}</div>;
}

/** Labelled text field. */
export function MobileField({
  label, className = "", ...props
}: InputHTMLAttributes<HTMLInputElement> & { label: string }) {
  return (
    <label className={`m-field ${className}`}>
      <span className="m-field__label">{label}</span>
      <input {...props} className="m-field__input" />
    </label>
  );
}

export function SectionHeader({ title, action }: { title: string; action?: ReactNode }) {
  return (
    <header className="m-section-header">
      <h2 className="m-section-header__title">{title}</h2>
      {action}
    </header>
  );
}

export function EmptyNote({ children }: { children: ReactNode }) {
  return <p className="m-empty">{children}</p>;
}
