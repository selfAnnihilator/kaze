// Fired when a cloud sync merged other devices' listening data, so open
// stats screens can reload without App state changes.
type Listener = () => void;
const listeners = new Set<Listener>();

export const cloudSyncSignal = {
  emit(): void {
    listeners.forEach((listener) => listener());
  },
  subscribe(listener: Listener): () => void {
    listeners.add(listener);
    return () => { listeners.delete(listener); };
  },
};
