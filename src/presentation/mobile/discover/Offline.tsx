import { useEffect, useState } from "react";
import { Download, WifiOff } from "lucide-react";
import { useMobileNav } from "../mobileNav";

/** True while the phone has a connection; follows the browser's online/offline events. */
export function useOnline() {
  const [online, setOnline] = useState(() => (typeof navigator === "undefined" ? true : navigator.onLine));
  useEffect(() => {
    const up = () => setOnline(true);
    const down = () => setOnline(false);
    window.addEventListener("online", up);
    window.addEventListener("offline", down);
    return () => { window.removeEventListener("online", up); window.removeEventListener("offline", down); };
  }, []);
  return online;
}

/** Discover offline (Figma C6): "No signal. Still music." and a way to downloaded songs. */
export function Offline() {
  const nav = useMobileNav();
  return (
    <div className="m-offline">
      <span className="m-offline__icon"><WifiOff size={28} strokeWidth={2} /></span>
      <span className="m-kicker">02 // DISCOVER IS OFFLINE</span>
      <h1 className="m-display m-offline__title">NO SIGNAL. STILL MUSIC.</h1>
      <p className="m-offline__text">Online music needs a connection. Everything you have downloaded keeps playing.</p>
      <button type="button" className="m-btn m-offline__go" onClick={() => nav.openLibrary("downloads")}>
        <Download size={18} strokeWidth={2} /><span>GO TO DOWNLOADED SONGS</span>
      </button>
    </div>
  );
}
