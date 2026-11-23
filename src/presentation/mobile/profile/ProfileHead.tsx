import { useRef, useState, type ChangeEvent } from "react";
import { User } from "lucide-react";
import { dispatchCommand } from "../../../services/api";
import type { ProfileStats } from "./useProfileStats";
import type { MobileScreenProps } from "../types";
import { shrinkPhoto } from "../../../services/photoUpload";

type Ctx = MobileScreenProps["c"];

const memberSince = (secs?: number) =>
  secs ? new Date(secs * 1000).toLocaleDateString("en-US", { month: "short", year: "numeric" }).toUpperCase() : null;

/** Change Photo / Remove: Android's own image picker, shrunk and sent as base64. */
function usePhoto(c: Ctx) {
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (send: () => Promise<{ data?: unknown } | undefined>) => {
    setBusy(true);
    setError(null);
    try {
      const res = await send();
      if (res?.data) c.setCurrentUser(res.data as Ctx["currentUser"]);
    } catch (e) {
      setError(typeof e === "string" ? e : "Couldn’t change your photo. Try another image.");
    } finally {
      setBusy(false);
    }
  };
  const picked = (e: ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (file) void run(async () => dispatchCommand({ command: "UploadAvatar", payload: { image_base64: await shrinkPhoto(file) } }));
  };
  return {
    input, busy, error, picked,
    choose: () => input.current?.click(),
    remove: () => void run(() => dispatchCommand({ command: "RemoveAvatar" })),
  };
}

/** Profile head (Figma F1): photo, name, member since, lifetime hours, Change Photo / Remove. */
export function ProfileHead({ c, p }: { c: Ctx; p: ProfileStats }) {
  const user = c.currentUser;
  const photo = usePhoto(c);
  const since = memberSince(user?.created_at ?? p.stats?.user_joined_date);
  return (
    <div className="m-profile-head">
      <div className={`m-profile-head__photo ${photo.busy ? "is-busy" : ""}`} aria-busy={photo.busy}>
        {user?.avatar_data_url ? <img src={user.avatar_data_url} alt="" /> : <User size={40} strokeWidth={2} />}
        {photo.busy && <span className="m-profile-head__buffer" aria-hidden="true"><span /></span>}
      </div>
      <div className="m-profile-head__text">
        <h1 className="m-display m-profile-head__name">{(user ? user.display_name || user.username : "Guest listener").toUpperCase()}</h1>
        {since && <span className="m-profile-meta">MEMBER SINCE {since}</span>}
        <span className="m-profile-meta is-red">{Math.floor((p.stats?.lifetime_seconds ?? 0) / 3600)}H LIFETIME</span>
        {user && (
          <span className="m-profile-head__links">
            <input ref={photo.input} type="file" accept="image/*" hidden onChange={photo.picked} aria-hidden="true" tabIndex={-1} />
            <button type="button" disabled={photo.busy} onClick={photo.choose}>{photo.busy ? "SAVING…" : "CHANGE PHOTO"}</button>
            {user.avatar_data_url && <button type="button" className="is-muted" disabled={photo.busy} onClick={photo.remove}>REMOVE</button>}
          </span>
        )}
        {photo.error && <span className="m-profile-meta is-red" role="alert">{photo.error}</span>}
      </div>
    </div>
  );
}
