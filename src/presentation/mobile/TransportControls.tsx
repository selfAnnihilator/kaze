import { memo } from "react";
import { Pause, Play, Repeat, Repeat1, SkipBack, SkipForward } from "lucide-react";
import { MobileIconButton } from "./primitives";

export interface TransportProps {
  isPlaying: boolean;
  isLoading: boolean;
  repeat: "off" | "one" | "all";
  onPlayPause: () => void;
  onNext: () => void;
  onPrevious: () => void;
  onToggleRepeat: () => void;
}

/** Previous, play/pause, next and repeat. */
export const TransportControls = memo(function TransportControls(p: TransportProps) {
  const RepeatIcon = p.repeat === "one" ? Repeat1 : Repeat;
  return (
    <div className="m-transport">
      <MobileIconButton label={`Repeat ${p.repeat}`} active={p.repeat !== "off"} onClick={p.onToggleRepeat}>
        <RepeatIcon size={20} />
      </MobileIconButton>
      <MobileIconButton label="Previous" onClick={p.onPrevious}><SkipBack size={24} /></MobileIconButton>
      <MobileIconButton label={p.isPlaying ? "Pause" : "Play"} className="m-transport__main"
        disabled={p.isLoading} onClick={p.onPlayPause}>
        {p.isPlaying ? <Pause size={28} /> : <Play size={28} />}
      </MobileIconButton>
      <MobileIconButton label="Next" onClick={p.onNext}><SkipForward size={24} /></MobileIconButton>
    </div>
  );
});
