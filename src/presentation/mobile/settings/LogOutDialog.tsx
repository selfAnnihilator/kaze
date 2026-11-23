import { useState } from "react";
import { Check, LogOut, MonitorSmartphone, Smartphone } from "lucide-react";
import { MobileDialog } from "../MobileDialog";

/** Log Out Choice (Figma G9): this device or all devices, then Cancel / Log Out. */
export function LogOutDialog({ onCancel, onLogOut }: { onCancel: () => void; onLogOut: (scope: "device" | "all") => Promise<void> }) {
  const [scope, setScope] = useState<"device" | "all">("device");
  const [busy, setBusy] = useState(false);
  const option = (value: "device" | "all", label: string, icon: React.ReactNode) => (
    <button type="button" className={`m-dialog__option ${scope === value ? "is-on" : ""}`} aria-pressed={scope === value} onClick={() => setScope(value)}>
      {icon}<span>{label}</span>{scope === value && <Check size={16} strokeWidth={2} />}
    </button>
  );
  const logOut = async () => {
    setBusy(true);
    try { await onLogOut(scope); } finally { setBusy(false); }
  };
  return (
    <MobileDialog kicker="ACCOUNT" title="LOG OUT WHERE?" onDismiss={busy ? undefined : onCancel}
      actions={<>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" disabled={busy} onClick={onCancel}>CANCEL</button>
        <button type="button" className="m-btn m-btn--sm m-btn--grow" disabled={busy} onClick={() => void logOut()}><LogOut size={18} strokeWidth={2} /><span>LOG OUT</span></button>
      </>}>
      {option("device", "This device", <Smartphone size={20} strokeWidth={2} />)}
      {option("all", "All devices", <MonitorSmartphone size={20} strokeWidth={2} />)}
    </MobileDialog>
  );
}
