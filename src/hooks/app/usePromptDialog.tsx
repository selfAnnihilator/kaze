import { lazy, Suspense, useCallback, useState } from "react";
import { PromptModal, type PromptModalProps } from "../../components/modals/PromptModal";
import { useInterfaceStyle } from "../../presentation/InterfaceContext";

const MobilePrompt = lazy(() => import("../../presentation/mobile/MobilePrompts").then((m) => ({ default: m.MobilePrompt })));

/** The phone gets the Figma G7 prompt; the PC keeps its own. */
function PromptSurface(props: PromptModalProps) {
  if (useInterfaceStyle() !== "mobile") return <PromptModal {...props} />;
  return <Suspense fallback={null}><MobilePrompt {...props} /></Suspense>;
}

interface PendingPrompt {
  title: string;
  message: string;
  defaultValue: string;
  resolve: (value: string | null) => void;
}

/** A themed text prompt (replaces window.prompt): `requestPrompt` resolves with the answer or null. */
export function usePromptDialog() {
  const [pending, setPending] = useState<PendingPrompt | null>(null);
  const requestPrompt = useCallback(
    (title: string, message: string, defaultValue: string = ""): Promise<string | null> =>
      new Promise((resolve) => setPending({ title, message, defaultValue, resolve })),
    [],
  );
  const answer = (value: string | null) => {
    pending?.resolve(value);
    setPending(null);
  };
  const promptDialog = pending && (
    <PromptSurface
      title={pending.title}
      message={pending.message}
      defaultValue={pending.defaultValue}
      onConfirm={(value) => answer(value)}
      onCancel={() => answer(null)}
    />
  );
  return { requestPrompt, promptDialog };
}
