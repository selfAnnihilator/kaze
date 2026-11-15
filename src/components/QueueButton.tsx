import { memo } from "react";
import { Check, ListPlus } from "lucide-react";

/** Adds a song to the end of the queue without interrupting what is playing. */
export const QueueButton = memo(function QueueButton({ title, queued, onEnqueue, size = 16 }: {
  title: string;
  queued: boolean;
  onEnqueue: () => void;
  size?: number;
}) {
  const label = queued ? `${title} is already in the queue` : `Add ${title} to queue`;
  return (
    <button
      type="button"
      className="player-icon-btn queue-button"
      aria-label={label}
      title={queued ? "Already in queue" : "Add to queue"}
      aria-pressed={queued}
      onClick={(event) => {
        event.stopPropagation();
        onEnqueue();
      }}
    >
      {queued ? <Check size={size} /> : <ListPlus size={size} />}
    </button>
  );
});
