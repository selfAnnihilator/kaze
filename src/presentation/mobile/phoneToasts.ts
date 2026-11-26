import { useSyncExternalStore } from "react";
import { queueIdOf } from "../../trackKind";
import type { AppNotification, DiscoveryRecommendation, ToastAction } from "../../types";
import type { MobileScreenProps } from "./types";

type Ctx = MobileScreenProps["c"];

/**
 * Phone-only toasts (Figma G10 "Added to queue · UNDO"). They stay out of the
 * notification inbox and out of App state, so showing one re-renders nothing else.
 */
let toasts: AppNotification[] = [];
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

export function showPhoneToast(title: string, action?: ToastAction, type: AppNotification["type"] = "success") {
  const id = `phone_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`;
  toasts = [...toasts, { id, type, title, message: "", timestamp: Date.now(), read: true, action }];
  emit();
}

export function dismissPhoneToast(id: string) {
  toasts = toasts.filter((t) => t.id !== id);
  emit();
}

export function usePhoneToasts(): AppNotification[] {
  return useSyncExternalStore((l) => { listeners.add(l); return () => listeners.delete(l); }, () => toasts);
}

const undo = (c: Ctx, trackId: string): ToastAction => ({ label: "UNDO", run: () => c.handleDequeueTrack(trackId) });

/** Adds a library song to the queue (or next), then offers UNDO. */
export async function queueTrack(c: Ctx, trackId: string, playNext = false) {
  if (await c.handleEnqueueTrack(trackId, playNext)) showPhoneToast(playNext ? "Playing next" : "Added to queue", undo(c, trackId));
}

/** Adds an online song to the queue, then offers UNDO. */
export async function queueRecommendation(c: Ctx, rec: DiscoveryRecommendation) {
  if (await c.handleEnqueueRecommendation(rec)) showPhoneToast("Added to queue", undo(c, queueIdOf(rec)));
}
