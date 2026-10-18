import { useEffect, useState } from "react";
export function useDebouncedValue<T>(value: T) {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), 200);
    return () => clearTimeout(timer);
  }, [value]);
  return debounced;
}
