import { createContext, useContext, useRef, type ReactNode } from "react";

const ViewMemoryContext = createContext<Map<string, unknown> | null>(null);

/** Small presentation snapshots survive route unmounts, without keeping views alive. */
export function ViewMemoryProvider({ children }: { children: ReactNode }) {
  const memory = useRef(new Map<string, unknown>());
  return (
    <ViewMemoryContext.Provider value={memory.current}>
      {children}
    </ViewMemoryContext.Provider>
  );
}

/** Mutable leaf-owned memory: scrolling never schedules App or chrome renders. */
export function useViewMemory<T>(name: string, create: () => T): T {
  const memory = useContext(ViewMemoryContext);
  const fallback = useRef<{ value: T } | null>(null);
  if (memory) {
    if (!memory.has(name)) memory.set(name, create());
    return memory.get(name) as T;
  }
  if (!fallback.current) fallback.current = { value: create() };
  return fallback.current.value;
}

export function useDiscoveryMemory() {
  return useViewMemory("discovery", () => ({
    chapter: 0,
    shelves: new Map<number, number>(),
    hovered: null as { chapter: number; id: string } | null,
  }));
}
