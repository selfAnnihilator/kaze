import { memo, useState } from "react";
import { UserRound } from "lucide-react";
import type { UserProfile } from "../types";
import type { ViewType } from "../components/Sidebar";

export const ProfileNavigation = memo(function ProfileNavigation({
  user,
  active,
  compact = true,
  onSelectView,
}: {
  user?: UserProfile | null;
  active: boolean;
  compact?: boolean;
  onSelectView: (view: ViewType) => void;
}) {
  const src = user?.avatar_data_url || user?.avatar_url;
  const [failedSrc, setFailedSrc] = useState<string>();
  const initial = (user?.display_name?.trim() || user?.username?.trim() || "")
    .charAt(0)
    .toUpperCase();
  return (
    <button
      type="button"
      className={`nav-button profile-navigation${active ? " active" : ""}`}
      aria-label="Profile"
      title="Profile and stats"
      aria-current={active ? "page" : undefined}
      onClick={() => onSelectView("stats")}
    >
      <span className="profile-navigation-avatar" aria-hidden="true">
        {src && src !== failedSrc ? (
          <img
            src={src}
            alt=""
            width={28}
            height={28}
            decoding="async"
            onError={() => setFailedSrc(src)}
          />
        ) : initial ? (
          <span>{initial}</span>
        ) : (
          <UserRound size={18} />
        )}
      </span>
      <span className={compact ? "spatial-sr-only" : undefined}>Profile</span>
    </button>
  );
});
