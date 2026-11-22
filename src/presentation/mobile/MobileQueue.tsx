import { memo } from "react";
import { X } from "lucide-react";
import { WindowedList } from "../WindowedList";
import { MobileButton, MobileIconButton, EmptyNote, SectionHeader } from "./primitives";
import { TrackRow } from "./TrackRow";
import type { QueueEntry } from "../queuePresentation";
import type { MobileScreenProps } from "./types";

const ROW_HEIGHT = 72;

/** Queue: the playing track, then what comes next. */
export const MobileQueue = memo(function MobileQueue({ c }: MobileScreenProps) {
  const current = c.playbackState.current_track;
  const renderEntry = (entry: QueueEntry) => (
    <TrackRow title={entry.title ?? "Up next"} subtitle={entry.artist} trackId={entry.trackId}
      onPress={() => c.handlePlayQueueIndex(entry.index)}
      trailing={<MobileIconButton label={`Remove ${entry.title ?? "track"}`}
        onClick={() => c.handleDequeueTrack(entry.trackId)}><X size={18} /></MobileIconButton>} />
  );
  return (
    <div className="m-screen__body m-screen__body--fill">
      <SectionHeader title="Playing" />
      {current ? (
        <TrackRow active title={current.title} subtitle={current.artist_name} trackId={current.id}
          durationSecs={current.duration_secs} onPress={() => void c.handleUnifiedPlayPause()} />
      ) : <EmptyNote>Nothing is playing.</EmptyNote>}
      <SectionHeader title="Up next" action={c.queueEntries.length ? (
        <MobileButton variant="ghost" onClick={c.handleClearQueue}>Clear</MobileButton>) : undefined} />
      {c.queueEntries.length
        ? <WindowedList items={c.queueEntries} renderItem={renderEntry} rowHeight={ROW_HEIGHT} label="Queue" fill />
        : <EmptyNote>The queue is empty.</EmptyNote>}
    </div>
  );
});
