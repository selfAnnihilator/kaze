import { useSyncExternalStore } from "react";

export interface ConfirmRequest {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
}

interface OpenConfirm extends ConfirmRequest {
  resolve: (confirmed: boolean) => void;
}

let open: OpenConfirm | null = null;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach(listener => listener());

/**
 * Asks a Yes/No question from anywhere (no App state, so no App re-render).
 * Resolves false when dismissed. A new question cancels an unanswered one.
 */
export function confirmAction(request: ConfirmRequest): Promise<boolean> {
  open?.resolve(false);
  return new Promise(resolve => {
    open = { ...request, resolve };
    emit();
  });
}

export function answerConfirm(confirmed: boolean) {
  const current = open;
  if (!current) return;
  open = null;
  emit();
  current.resolve(confirmed);
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
};
const snapshot = () => open;

export function useConfirmRequest(): ConfirmRequest | null {
  return useSyncExternalStore(subscribe, snapshot, snapshot);
}
