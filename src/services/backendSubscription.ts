/** Own one listener and one registration attempt, including late cleanup. */
export function ownBackendSubscription(
  register: () => Promise<() => void>,
  connected: () => void,
  status: (failed: boolean) => void,
) {
  let disposed = false, pending = false, listener: (() => void) | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let failures = 0;
  const attempt = async () => {
    if (disposed || pending || listener) return;
    clearTimeout(timer);
    pending = true;
    try {
      const cleanup = await register();
      if (disposed) { cleanup(); return; }
      listener = cleanup;
      failures = 0;
      status(false);
      connected();
    } catch {
      if (disposed) return;
      if (failures < 3) timer = setTimeout(() => void attempt(), 1000 * 2 ** failures++);
      else status(true);
    } finally { pending = false; }
  };
  void attempt();
  return {
    retry() { failures = 0; void attempt(); },
    dispose() { disposed = true; clearTimeout(timer); listener?.(); listener = undefined; },
  };
}
