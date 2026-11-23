import "./mobile-notices.css";
import { memo, useEffect, useState, type ComponentProps, type ReactNode } from "react";
import { ArrowUpCircle, Check, CircleAlert, CirclePlay, X } from "lucide-react";
import { recommendationNoticeText, undoRecommendationFeedback, type RecommendationNotice } from "../../components/RecommendationNotice";
import { useAvailableUpdate } from "../../components/UpdateBanner";
import { startAppUpdate } from "./appUpdate";
import { dismissPhoneToast, usePhoneToasts } from "./phoneToasts";
import type { ToastContainer } from "../../components/notifications/ToastContainer";
import type { AppNotification } from "../../types";

const PLAYBACK_EVENT = "kaze-empty-playback";

/** Update banner (Figma G10): red bar on top with the version; the app restarts itself once it is in. */
export const MobileUpdateBanner = memo(function MobileUpdateBanner() {
  const version = useAvailableUpdate();
  const [closed, setClosed] = useState(false);
  if (!version || closed) return null;
  return (
    <div className="m-update" role="status">
      <ArrowUpCircle size={20} strokeWidth={2} />
      <span className="m-notice__text"><strong>Kaze v{version} is available</strong><span className="m-notice__kind">NEW VERSION</span></span>
      <button type="button" className="m-update__btn" onClick={() => startAppUpdate(version)}>UPDATE</button>
      <button type="button" className="m-notice__x" onClick={() => setClosed(true)} aria-label="Dismiss update"><X size={18} strokeWidth={2} /></button>
    </div>
  );
});

/** Recommendation notice (Figma G10): red bar, what Kaze won't recommend, Allow again / Undo, close. */
export function MobileRecommendationNotice({ notice, onDismiss, onError }: ComponentProps<typeof RecommendationNotice>) {
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (busy) return;
    const timer = setTimeout(() => onDismiss(notice), 5000);
    return () => clearTimeout(timer);
  }, [notice, busy, onDismiss]);
  const undo = async () => {
    setBusy(true);
    try { await undoRecommendationFeedback(notice); onDismiss(notice); } catch (e) { onError(String(e)); setBusy(false); }
  };
  return (
    <div className="m-notice" role="status">
      <span className="m-notice__bar" />
      <span className="m-notice__text"><strong>{recommendationNoticeText(notice)}</strong><span className="m-notice__kind">RECOMMENDATION NOTICE</span></span>
      <button type="button" className="m-notice__action" disabled={busy} onClick={() => void undo()}>{notice.entities?.length ? "UNDO" : "ALLOW AGAIN"}</button>
      <button type="button" className="m-notice__x" disabled={busy} onClick={() => onDismiss(notice)} aria-label="Dismiss"><X size={16} strokeWidth={2} /></button>
    </div>
  );
}

/** Playback notice (Figma G10): "Select a song to play" when play is pressed with nothing queued. */
export const MobilePlaybackNotice = memo(function MobilePlaybackNotice() {
  const [shown, setShown] = useState(0);
  useEffect(() => {
    const show = () => setShown((n) => n + 1);
    window.addEventListener(PLAYBACK_EVENT, show);
    return () => window.removeEventListener(PLAYBACK_EVENT, show);
  }, []);
  useEffect(() => {
    if (!shown) return;
    const timer = setTimeout(() => setShown(0), 2000);
    return () => clearTimeout(timer);
  }, [shown]);
  if (!shown) return null;
  return (
    <div className="m-notice" role="status">
      <CirclePlay size={20} strokeWidth={2} />
      <span className="m-notice__text"><strong>Select a song to play</strong><span className="m-notice__kind">PLAYBACK NOTICE</span></span>
    </div>
  );
});

function Toast({ toast, onDismiss }: { toast: AppNotification; onDismiss: (id: string) => void }) {
  useEffect(() => {
    const timer = setTimeout(() => onDismiss(toast.id), 3000);
    return () => clearTimeout(timer);
  }, [toast.id, onDismiss]);
  const failed = toast.type === "error" || toast.type === "warning";
  return (
    <button type="button" className={`m-toast ${failed ? "is-error" : ""}`} onClick={() => onDismiss(toast.id)} aria-label={`${toast.title}. Dismiss`}>
      {failed ? <CircleAlert size={18} strokeWidth={2} /> : <Check size={18} strokeWidth={2} />}
      <span className="m-toast__title">{toast.title || toast.message}</span>
      {toast.action && (
        <span role="button" tabIndex={0} className="m-toast__action"
          onClick={(e) => { e.stopPropagation(); toast.action!.run(); onDismiss(toast.id); }}
          onKeyDown={(e) => { if (e.key === "Enter") { e.stopPropagation(); toast.action!.run(); onDismiss(toast.id); } }}>
          {toast.action.label}
        </span>
      )}
    </button>
  );
}

/** Toasts (Figma G10): ink bar for done, red bar for failed; above the mini deck. */
export function MobileToasts({ toasts, onDismiss, children }: ComponentProps<typeof ToastContainer> & { children?: ReactNode }) {
  const phone = usePhoneToasts();
  return (
    <div className="m-toasts" aria-live="polite">
      {children}
      {toasts.slice(-3).map((t) => <Toast key={t.id} toast={t} onDismiss={onDismiss} />)}
      {phone.slice(-2).map((t) => <Toast key={t.id} toast={t} onDismiss={dismissPhoneToast} />)}
    </div>
  );
}
