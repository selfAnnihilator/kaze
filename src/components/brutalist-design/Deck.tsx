import { Dispatch, memo, useState } from "react";
import {
  ArrowUpRight,
  Heart,
  Pause,
  Play,
  Repeat2,
  Shuffle,
  SkipBack,
  SkipForward,
  Volume2,
  VolumeX,
  Wind,
} from "lucide-react";
import { Action, DeckState, Track, formatTime } from "./model";

const Position = memo(function Position({ track }: { track: Track }) {
  const [position, setPosition] = useState(0);
  return (
    <div className="bk-position">
      <label htmlFor="bk-position">
        POSITION{" "}
        <output>
          {formatTime(position)} / {formatTime(track.duration)}
        </output>
      </label>
      <input
        id="bk-position"
        type="range"
        min={0}
        max={track.duration}
        value={position}
        onChange={(event) => setPosition(Number(event.target.value))}
        aria-label="Seek sample track"
      />
    </div>
  );
});
const Volume = memo(function Volume() {
  const [value, setValue] = useState(70);
  const [muted, setMuted] = useState(false);
  return (
    <div className="bk-volume">
      <button
        className="bk-icon"
        onClick={() => setMuted((current) => !current)}
        aria-label={muted ? "Unmute sample volume" : "Mute sample volume"}
      >
        {muted ? <VolumeX size={20} /> : <Volume2 size={20} />}
      </button>
      <input
        type="range"
        min={0}
        max={100}
        value={muted ? 0 : value}
        onChange={(event) => {
          setValue(Number(event.target.value));
          setMuted(false);
        }}
        aria-label="Sample volume"
      />
      <output>{muted ? "00" : String(value).padStart(2, "0")}</output>
    </div>
  );
});
export const Deck = memo(function Deck({
  state,
  dispatch,
  onQueue,
  onPlay,
}: {
  state: DeckState;
  dispatch: Dispatch<Action>;
  onQueue: () => void;
  onPlay: (id: string) => void;
}) {
  const track = state.queue[state.cursor];
  const ahead = state.queue.slice(state.cursor + 1, state.cursor + 4);
  return (
    <aside className="bk-deck" aria-label="Playback controls">
      <div className="bk-deck-label">
        <span>THE DECK</span>
        <span>
          {String(state.cursor + 1).padStart(2, "0")} /{" "}
          {String(state.queue.length).padStart(2, "0")}
        </span>
      </div>
      <div className="bk-deck-track" aria-live="polite">
        <span className="bk-micro">
          {state.playing ? "PLAYING IN PREVIEW" : "PAUSED IN PREVIEW"}
        </span>
        <h2>{track.title}</h2>
        <p>{track.artist}</p>
      </div>
      <button
        className="bk-play-main"
        onClick={() => dispatch({ type: "toggle" })}
        aria-label={state.playing ? "Pause sample track" : "Play sample track"}
      >
        {state.playing ? (
          <Pause size={42} fill="currentColor" />
        ) : (
          <Play size={42} fill="currentColor" />
        )}
        <span>{state.playing ? "PAUSE" : "PLAY"}</span>
      </button>
      <div className="bk-skips">
        <button
          onClick={() => dispatch({ type: "previous" })}
          disabled={state.cursor === 0}
          aria-label="Previous track"
        >
          <SkipBack size={21} />
          <span>PREV</span>
        </button>
        <button
          onClick={() => dispatch({ type: "next" })}
          disabled={!state.repeat && state.cursor === state.queue.length - 1}
          aria-label="Next track"
        >
          <span>NEXT</span>
          <SkipForward size={21} />
        </button>
      </div>
      <Position key={track.id} track={track} />
      <div className="bk-deck-options">
        <button
          className="bk-icon"
          aria-label="Shuffle upcoming tracks"
          aria-pressed={state.shuffle}
          onClick={() => dispatch({ type: "shuffle" })}
        >
          <Shuffle size={20} />
        </button>
        <button
          className="bk-icon"
          aria-label="Repeat queue"
          aria-pressed={state.repeat}
          onClick={() => dispatch({ type: "repeat" })}
        >
          <Repeat2 size={20} />
        </button>
        <button
          className="bk-icon"
          aria-label={
            state.liked.includes(track.id)
              ? "Unlike current track"
              : "Like current track"
          }
          aria-pressed={state.liked.includes(track.id)}
          onClick={() => dispatch({ type: "like" })}
        >
          <Heart
            size={20}
            fill={state.liked.includes(track.id) ? "currentColor" : "none"}
          />
        </button>
      </div>
      <div className="bk-deck-next">
        <button className="bk-deck-next-heading" onClick={onQueue}>
          UP NEXT <ArrowUpRight size={18} />
        </button>
        {ahead.length ? (
          ahead.map((item, index) => (
            <button
              key={item.id}
              className="bk-deck-next-song"
              onClick={() => onPlay(item.id)}
            >
              <span>{String(index + 1).padStart(2, "0")}</span>
              <strong>
                {item.title}
                <small>{item.artist}</small>
              </strong>
              <ArrowUpRight size={16} />
            </button>
          ))
        ) : (
          <p className="bk-next-empty">
            End of the queue. Find your next record.
          </p>
        )}
      </div>
      <div className="bk-deck-bottom">
        <button
          className="bk-flow"
          aria-label={state.flow ? "Stop sample Flow" : "Start sample Flow"}
          aria-pressed={state.flow}
          onClick={() => dispatch({ type: "flow" })}
        >
          <Wind size={23} />
          <span>FLOW</span>
          <small>{state.flow ? "ON" : "OFF"}</small>
        </button>
        <Volume />
        <p className="bk-preview-note">DESIGN STUDY / NO AUDIO</p>
      </div>
    </aside>
  );
});
