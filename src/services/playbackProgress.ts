import { useState, useEffect } from "react";

type ProgressCallback = (position: number, duration: number) => void;

class PlaybackProgressEmitter {
  private listeners: Set<ProgressCallback> = new Set();
  private currentPosition: number = 0;
  private currentDuration: number = 0;

  public subscribe(callback: ProgressCallback): () => void {
    this.listeners.add(callback);
    // Notify immediately with current values
    callback(this.currentPosition, this.currentDuration);
    return () => {
      this.listeners.delete(callback);
    };
  }

  public update(position: number, duration: number): void {
    this.currentPosition = position;
    this.currentDuration = duration;
    for (const listener of this.listeners) {
      listener(this.currentPosition, this.currentDuration);
    }
  }

  public getPosition(): number {
    return this.currentPosition;
  }

  public getDuration(): number {
    return this.currentDuration;
  }
}

export const playbackProgress = new PlaybackProgressEmitter();

export function usePlaybackProgress(_fallbackDuration = 0) {
  const [progress, setProgress] = useState(() => ({
    position: playbackProgress.getPosition(),
    duration: playbackProgress.getDuration(),
  }));

  useEffect(() => {
    const update = (position: number, duration: number) => {
      if (document.hidden) return;
      const nextDuration = duration;
      setProgress((current) =>
        current.position === position && current.duration === nextDuration
          ? current
          : { position, duration: nextDuration },
      );
    };
    const unsubscribe = playbackProgress.subscribe(update);
    const restore = () =>
      update(playbackProgress.getPosition(), playbackProgress.getDuration());
    document.addEventListener("visibilitychange", restore);
    return () => {
      unsubscribe();
      document.removeEventListener("visibilitychange", restore);
    };
  }, []);

  return progress;
}

/**
 * A value derived from the position, re-rendering only when it changes (for
 * example the active lyric line, which changes every few seconds rather than
 * on every 4 Hz tick). `select` must be stable (useCallback).
 */
export function usePlaybackSelector<T>(select: (position: number, duration: number) => T): T {
  const [value, setValue] = useState(() =>
    select(playbackProgress.getPosition(), playbackProgress.getDuration()),
  );
  useEffect(() => {
    const update = (position: number, duration: number) => {
      if (document.hidden) return;
      const next = select(position, duration);
      setValue((current) => (Object.is(current, next) ? current : next));
    };
    const unsubscribe = playbackProgress.subscribe(update);
    const restore = () =>
      update(playbackProgress.getPosition(), playbackProgress.getDuration());
    document.addEventListener("visibilitychange", restore);
    return () => {
      unsubscribe();
      document.removeEventListener("visibilitychange", restore);
    };
  }, [select]);
  return value;
}
