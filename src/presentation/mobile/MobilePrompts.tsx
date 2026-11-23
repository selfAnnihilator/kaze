import { useState } from "react";
import { Trash2 } from "lucide-react";
import { answerConfirm, useConfirmRequest } from "../../services/confirmDialog";
import { MobileDialog } from "./MobileDialog";
import type { PromptModalProps } from "../../components/modals/PromptModal";

/** Text prompt (Figma G7 "NAME IT."): used to name or rename a playlist. */
export function MobilePrompt({ title, message, defaultValue = "", onConfirm, onCancel }: PromptModalProps) {
  const [value, setValue] = useState(defaultValue);
  const creating = /new/i.test(title);
  const submit = () => { if (value.trim()) onConfirm(value.trim()); };
  const extra = message && !/^name it\.?$/i.test(message.trim()) ? message : null;
  return (
    <MobileDialog kicker={title.toUpperCase()} title="NAME IT." onDismiss={onCancel}
      actions={<>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" onClick={onCancel}>CANCEL</button>
        <button type="button" className="m-btn m-btn--sm m-btn--grow" disabled={!value.trim()} onClick={submit}>{creating ? "CREATE" : "SAVE"}</button>
      </>}>
      {extra && <p className="m-dialog__text">{extra}</p>}
      <form onSubmit={(e) => { e.preventDefault(); submit(); }}>
        <input className="m-auth__input m-dialog__input" value={value} onChange={(e) => setValue(e.target.value)} autoFocus aria-label={title} enterKeyHint="done" />
      </form>
    </MobileDialog>
  );
}

/** Yes/No question (Figma G8): Keep / the action, for anything `confirmAction()` asks. */
export function MobileConfirm() {
  const request = useConfirmRequest();
  if (!request) return null;
  const destructive = /delete|remove/i.test(`${request.title} ${request.confirmLabel ?? ""}`);
  return (
    <MobileDialog kicker="CONFIRM" title={request.title.toUpperCase()} onDismiss={() => answerConfirm(false)}
      actions={<>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" onClick={() => answerConfirm(false)}>{(request.cancelLabel || "Keep").toUpperCase()}</button>
        <button type="button" className="m-btn m-btn--sm m-btn--grow" onClick={() => answerConfirm(true)} autoFocus>
          {destructive && <Trash2 size={18} strokeWidth={2} />}<span>{(request.confirmLabel || "OK").toUpperCase()}</span>
        </button>
      </>}>
      <p className="m-dialog__text">{request.message}</p>
    </MobileDialog>
  );
}
