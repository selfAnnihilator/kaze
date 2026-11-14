import { useCallback, useState } from "react";
import { PromptModal } from "../../components/modals/PromptModal";

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
    <PromptModal
      title={pending.title}
      message={pending.message}
      defaultValue={pending.defaultValue}
      onConfirm={(value) => answer(value)}
      onCancel={() => answer(null)}
    />
  );
  return { requestPrompt, promptDialog };
}
