import { memo } from "react";
import { AlertCircle, CheckCircle2, Download, RotateCw } from "lucide-react";
import { type DownloadControlStatus, useDownloadAction } from "../hooks/useDownloadAction";

export { removeDownload } from "../services/downloadActions";

const LABELS: Record<DownloadControlStatus, (title: string, percent: number) => string> = {
  pending: title => `Checking download status of ${title}`,
  removing: title => `Removing ${title} from downloads`,
  complete: title => `Downloaded ${title}. Click to remove`,
  active: (title, percent) => `Downloading ${title}: ${percent}%`,
  failed: title => `Retry download of ${title}`,
  idle: title => `Download ${title}`,
};

function DownloadIcon({ status }: { status: DownloadControlStatus }) {
  if (status === "removing" || status === "active") return <RotateCw size={14} className="spin-animation" />;
  if (status === "complete") return <CheckCircle2 size={16} />;
  if (status === "failed") return <AlertCircle size={16} />;
  return <Download size={16} />;
}

function DownloadText({ status, percent, compact }: { status: DownloadControlStatus; percent: number; compact: boolean }) {
  if (status === "active") return <span className="song-download-percent">{percent}%</span>;
  if (compact || (status !== "failed" && status !== "idle")) return null;
  return <span>{status === "failed" ? "Retry" : "Download"}</span>;
}

interface DownloadButtonProps {
  trackId: string;
  sourceId?: string;
  title: string;
  compact?: boolean;
  downloaded?: boolean;
  /** Track details still loading: show a neutral, disabled control. */
  pending?: boolean;
}

/** Download a song, show its progress, or (when downloaded) remove it after asking. */
export const DownloadButton = memo(function DownloadButton({ trackId, sourceId, title, compact = false, downloaded = false, pending = false }: DownloadButtonProps) {
  const { status, failed, error, percent, run } = useDownloadAction({ trackId, sourceId, title, downloaded, pending });
  const label = LABELS[status](title, percent);
  const busy = status === "removing" || status === "active";
  const className = `song-download-button ${compact ? "is-compact" : ""} ${status === "complete" ? "is-complete" : ""} ${failed ? "is-failed" : ""}`;
  return <span className="song-download-control">
    <button data-track-id={trackId} type="button" className={className} title={failed ? error : label} aria-label={label}
      disabled={pending || busy} aria-busy={pending || status === "removing" || undefined}
      onClick={event => { event.stopPropagation(); void run(); }}>
      {status !== "complete" && status !== "removing" && <span className="song-download-fill" style={{ transform: `scaleX(${percent / 100})` }} />}
      <span className="song-download-label"><DownloadIcon status={status} /><DownloadText status={status} percent={percent} compact={compact} /></span>
    </button>
    {failed && <span className={compact ? "sr-only" : "song-download-error"} role="alert">{error}</span>}
  </span>;
});
