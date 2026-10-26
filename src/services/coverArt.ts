import { executeQuery } from "./api";

type Reader = { done: (url: string | null) => void; detach: () => void };
type Request = {
  id: string;
  generation: number;
  readers: Set<Reader>;
  started: boolean;
};
const listeners = new Set<() => void>();
export function subscribeCoverArt(listener: () => void) { listeners.add(listener); return () => { listeners.delete(listener); }; }
export function coverArtGeneration() { return generation; }

const cache = new Map<string, string | null>();
const requests = new Map<string, Request>();
const queue: Request[] = [];
let bytes = 0,
  active = 0,
  generation = 0;
const MAX_BYTES = 8 * 1024 * 1024;

export function peekCoverArt(id: string): string | null | undefined {
  const value = cache.get(id);
  if (cache.has(id)) {
    cache.delete(id);
    cache.set(id, value ?? null);
  }
  return value;
}

export function clearCoverArtCache() {
  generation++;
  cache.clear();
  bytes = 0;
  for (const listener of listeners) listener();
}

function remember(id: string, url: string | null) {
  const cost = (url?.length || 0) * 2;
  if (cost > MAX_BYTES) return;
  bytes -= (cache.get(id)?.length || 0) * 2;
  cache.delete(id);
  cache.set(id, url);
  bytes += cost;
  while (cache.size > 64 || bytes > MAX_BYTES) {
    const oldest = cache.keys().next().value!;
    bytes -= (cache.get(oldest)?.length || 0) * 2;
    cache.delete(oldest);
  }
}

function drain() {
  while (active < 4 && queue.length) {
    const request = queue.shift()!;
    if (!request.readers.size) continue;
    active++;
    request.started = true;
    executeQuery({
      query: "GetTrackCoverArt",
      payload: { track_id: request.id },
    })
      .then(
        (response) => {
          const url =
            typeof response.data === "string" && response.data
              ? response.data
              : null;
          if (request.generation === generation) remember(request.id, url);
          return url;
        },
        () => null,
      )
      .then((url) => {
        for (const reader of request.readers) {
          reader.detach();
          reader.done(url);
        }
      })
      .finally(() => {
        if (requests.get(request.id) === request) requests.delete(request.id);
        active--;
        drain();
      });
  }
}

/** Deduplicate concurrent consumers; discard queued work when its view leaves. */
export function loadCoverArt(
  id: string,
  signal?: AbortSignal,
): Promise<string | null> {
  if (signal?.aborted) return Promise.resolve(null);
  const cached = peekCoverArt(id);
  if (cached !== undefined) return Promise.resolve(cached);
  let request = requests.get(id);
  if (!request || request.generation !== generation) {
    request = { id, generation, readers: new Set(), started: false };
    requests.set(id, request);
    queue.push(request);
  }
  const pending = request;
  return new Promise((resolve) => {
    const abort = () => {
      pending.readers.delete(reader);
      reader.detach();
      resolve(null);
      if (!pending.started && !pending.readers.size) {
        const index = queue.indexOf(pending);
        if (index >= 0) queue.splice(index, 1);
        if (requests.get(id) === pending) requests.delete(id);
      }
    };
    const reader: Reader = {
      done: resolve,
      detach: () => signal?.removeEventListener("abort", abort),
    };
    pending.readers.add(reader);
    signal?.addEventListener("abort", abort, { once: true });
    drain();
  });
}
