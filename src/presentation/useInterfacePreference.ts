import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { dispatchCommand } from "../services/api";
import type { AppSettings } from "../types";
import type { InterfaceStyle } from "./InterfaceContext";

export function useInterfacePreference(settings: AppSettings | null) {
  const [style, setStyle] = useState<InterfaceStyle>(
    settings?.interface_style ?? "spatial",
  );
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pending = useRef(false);
  useEffect(() => {
    if (!pending.current && settings?.interface_style)
      setStyle(settings.interface_style);
  }, [settings?.interface_style]);
  useLayoutEffect(() => {
    document.documentElement.dataset.interface = style;
  }, [style]);
  const select = useCallback(
    async (next: InterfaceStyle) => {
      if (pending.current || next === style) return;
      const previous = style;
      pending.current = true;
      setError(null);
      setSaving(true);
      setStyle(next);
      try {
        await dispatchCommand({
          command: "SetInterfaceStyle",
          payload: { style: next },
        });
      } catch {
        setStyle(previous);
        setError(
          "Couldn’t save Interface Style. Your previous style has been restored. Try again.",
        );
      } finally {
        pending.current = false;
        setSaving(false);
      }
    },
    [style],
  );
  return { style, saving, error, select };
}
