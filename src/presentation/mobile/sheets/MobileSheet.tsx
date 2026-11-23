import type { ReactNode } from "react";
import { X } from "lucide-react";
import { Sleeve } from "../Sleeve";

/**
 * Bottom sheet (Figma G1–G4): the screen fades back, then a paper sheet with a
 * red top bar, kicker and close, the song, and the sheet's rows.
 */
export function MobileSheet({ kicker, onClose, children, song }: {
  kicker: string; onClose: () => void; children: ReactNode;
  song?: { title: string; subtitle?: string; cover?: string | null; trackId?: string };
}) {
  return (
    <div className="m-bsheet" role="presentation">
      <div className="m-bsheet__dim" onClick={onClose} />
      <section className="m-bsheet__card" role="dialog" aria-modal="true" aria-label={kicker}>
        <div className="m-bsheet__bar" />
        <header className="m-bsheet__head">
          <span className="m-kicker">{kicker}</span>
          <button type="button" className="m-bsheet__close" onClick={onClose} aria-label="Close"><X size={20} strokeWidth={2} /></button>
        </header>
        {song && (
          <div className="m-bsheet__song">
            <Sleeve size="edge" src={song.cover} trackId={song.trackId} title={song.title} />
            <span className="m-bsheet__song-text"><strong>{song.title}</strong>{song.subtitle && <span>{song.subtitle}</span>}</span>
          </div>
        )}
        <div className="m-bsheet__body">{children}</div>
      </section>
    </div>
  );
}

/** One sheet row: icon, label, then an optional mono note or trailing mark. */
export function SheetRow({ icon, label, note, trailing, tone, disabled, onPress }: {
  icon: ReactNode; label: string; note?: string; trailing?: ReactNode; tone?: "red"; disabled?: boolean; onPress: () => void;
}) {
  return (
    <button type="button" className={`m-bsheet__row ${tone === "red" ? "is-red" : ""}`} disabled={disabled} onClick={onPress}>
      {icon}<strong>{label}</strong>{note && <span className="m-profile-meta">{note}</span>}{trailing}
    </button>
  );
}

export const joinSub = (...parts: Array<string | undefined | null>) => parts.filter(Boolean).join(" · ");
