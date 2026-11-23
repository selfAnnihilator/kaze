import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import type { HomeOverview, PlaybackState } from "../../types";

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;
import type { PlayingOrigin } from "../../playingOrigin";

/**
 * Home overview + Flow: the home feed, the Flow session (start, continue/pause,
 * debounced steering with revision guards), and the "why this track" modal.
 */
export function useFlow({
  libraryAccountRef, playbackState, fetchPlaybackState, onPlayPause, addAppNotification, setPlayingOrigin,
}: {
  libraryAccountRef: MutableRefObject<string>;
  playbackState: PlaybackState;
  fetchPlaybackState: () => Promise<unknown>;
  onPlayPause: () => Promise<void>;
  addAppNotification: Notify;
  setPlayingOrigin: Dispatch<SetStateAction<PlayingOrigin>>;
}) {
  const [homeOverview, setHomeOverview] = useState<HomeOverview | null>(null);
  const [isHomeLoading, setIsHomeLoading] = useState<boolean>(false);
  const [flowPendingAction, setFlowPendingAction] = useState<"start" | "resume" | "pause" | null>(null);
  const flowCommandPendingRef = useRef(false);
  const [flowSteering, setFlowSteering] = useState<number>(0.5);

  const homeRequestRef = useRef(0);
  const steeringRevisionRef = useRef(0);
  const steeringPendingRef = useRef(false);
  const flowSessionRequestRef = useRef(0);
  const fetchHomeOverview = useCallback(async () => {
    const account = libraryAccountRef.current;
    const request = ++homeRequestRef.current;
    const steeringRevision = steeringRevisionRef.current;
    const flowRevision = flowSessionRequestRef.current;
    try {
      setIsHomeLoading(true);
      const res = await executeQuery({ query: "GetHomeOverview" });
      if (res.type === "HomeOverview" && res.data && account === libraryAccountRef.current && request === homeRequestRef.current) {
        setHomeOverview(
          (prev) => ({ ...res.data, flow_session: flowRevision === flowSessionRequestRef.current ? res.data.flow_session : prev?.flow_session,
            }) as HomeOverview,
        );
        if (res.data.flow_session && !steeringPendingRef.current && steeringRevision === steeringRevisionRef.current) {
          setFlowSteering(res.data.flow_session.familiarity_ratio);
        }
      }
    } catch (err) {
      console.error("Failed to fetch home overview:", err);
    } finally {
      if (request === homeRequestRef.current) setIsHomeLoading(false);
    }
  }, []);

  const fetchFlowSession = useCallback(async () => {
    const request = ++flowSessionRequestRef.current;
    const res = await executeQuery({ query: "GetFlowSession" });
    if (res.type === "FlowSession" && request === flowSessionRequestRef.current) {
      setHomeOverview((prev) => ({ ...(prev || { recent_rotation: [], rediscover: [], for_you: [] }), flow_session: res.data,
      }));
      setPlayingOrigin((prev) => res.data?.is_active ? { type: "flow", name: "Flow" } : prev?.type === "flow" ? null : prev,
      );
    }
  }, []);

  const handleStartFlow = useCallback(
    async (familiarityRatio: number) => {
      if (flowCommandPendingRef.current) return;
      flowCommandPendingRef.current = true;
      setFlowPendingAction("start");
      try {
        await dispatchCommand({
          command: "StartFlow",
          payload: { familiarity_ratio: familiarityRatio },
        });
        await fetchFlowSession();
        // Home artwork and recommendations must not hold the playback action busy.
        void Promise.allSettled([fetchPlaybackState(), fetchHomeOverview()]);
      } catch (err) {
        console.error("Failed to start Flow:", err);
        addAppNotification(
          "error",
          "Flow Error",
          "Failed to start Flow: " + String(err),
        );
      } finally {
        flowCommandPendingRef.current = false;
        setFlowPendingAction(null);
      }
    },
    [
      fetchPlaybackState,
      fetchHomeOverview,
      fetchFlowSession,
      addAppNotification,
    ],
  );

  const handleToggleFlowPlayback = useCallback(async () => {
    if (flowCommandPendingRef.current) return;
    flowCommandPendingRef.current = true;
    setFlowPendingAction(playbackState.is_playing ? "pause" : "resume");
    try {
      if (playbackState.is_playing) await onPlayPause();
      else {
        await dispatchCommand({ command: "ContinueFlow" });
        void fetchPlaybackState();
      }
    } catch (error) {
      addAppNotification("error", "Flow Error", String(error));
    } finally {
      flowCommandPendingRef.current = false;
      setFlowPendingAction(null);
    }
  }, [playbackState.is_playing, onPlayPause, fetchPlaybackState, addAppNotification]);

  const steeringDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const handleFlowSteeringChange = useCallback((val: number) => {
    const revision = ++steeringRevisionRef.current;
    steeringPendingRef.current = true;
    setFlowSteering(val);
    if (steeringDebounceRef.current) {
      clearTimeout(steeringDebounceRef.current);
    }
    steeringDebounceRef.current = setTimeout(async () => {
      try {
        await dispatchCommand({
          command: "SetFlowSteering",
          payload: { familiarity_ratio: val },
        });
        if (revision === steeringRevisionRef.current) {
          steeringPendingRef.current = false;
          await fetchFlowSession();
          await fetchPlaybackState();
        }
      } catch (err) {
        console.error("Failed to update Flow steering:", err);
        if (revision === steeringRevisionRef.current)
          steeringPendingRef.current = false;
      }
    }, 250);
  }, [fetchFlowSession, fetchPlaybackState]);

  useEffect(
    () => () => {
      if (steeringDebounceRef.current)
        clearTimeout(steeringDebounceRef.current);
    },
    [],
  );

  return useMemo(() => ({
    homeOverview, setHomeOverview, isHomeLoading, flowPendingAction, flowSteering, homeRequestRef, flowSessionRequestRef, fetchHomeOverview, fetchFlowSession, handleStartFlow, handleToggleFlowPlayback, handleFlowSteeringChange,
  }), [homeOverview, setHomeOverview, isHomeLoading, flowPendingAction, flowSteering, homeRequestRef, flowSessionRequestRef, fetchHomeOverview, fetchFlowSession, handleStartFlow, handleToggleFlowPlayback, handleFlowSteeringChange]);
}
