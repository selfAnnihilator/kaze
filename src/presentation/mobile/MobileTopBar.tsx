import { memo } from "react";
import { Bell, Search, Settings, User } from "lucide-react";
import { useMobileNav } from "./mobileNav";

/** The uploaded photo, else initials, else the guest icon. */
function avatarTile(nav: ReturnType<typeof useMobileNav>) {
  if (nav.initials && nav.avatarUrl) return <img className="m-topbar__avatar-img" src={nav.avatarUrl} alt="" />;
  return nav.initials ?? <User size={16} strokeWidth={2} />;
}

export interface MobileTopBarProps {
  num: string;
  name: string;
}

/**
 * Brutalist Mobile Top Bar (Figma): "KAZE" with the screen tag, then Search
 * (Discover search), Notifications with the unread count, and the profile tile
 * (the uploaded photo or initials when signed in; Sign In when not). Signed out,
 * a Settings gear sits left of the tile, since Profile is unreachable then.
 */
export const MobileTopBar = memo(function MobileTopBar({ num, name }: MobileTopBarProps) {
  const nav = useMobileNav();
  return (
    <header className="m-topbar">
      <div className="m-topbar__brand">
        <span className="m-topbar__logo">KAZE</span>
        <span className="m-topbar__tag">{num} {name}</span>
      </div>
      <div className="m-topbar__actions">
        <button type="button" className="m-topbar__btn" onClick={nav.openSearch} aria-label="Search">
          <Search size={20} strokeWidth={2} />
        </button>
        <button type="button" className="m-topbar__btn" onClick={() => nav.openPage("notifications")}
          aria-label={nav.unread ? `Notifications, ${nav.unread} unread` : "Notifications"}>
          <Bell size={20} strokeWidth={2} />
          {nav.unread > 0 && <span className="m-topbar__badge">{nav.unread > 9 ? "9+" : nav.unread}</span>}
        </button>
        {!nav.initials && (
          <button type="button" className="m-topbar__btn" onClick={() => nav.openPage("settings")} aria-label="Settings">
            <Settings size={20} strokeWidth={2} />
          </button>
        )}
        <button type="button" className="m-topbar__avatar-btn" onClick={nav.openProfile}
          aria-label={nav.initials ? "Profile" : "Sign in or create account"}>
          <span className="m-topbar__avatar-tile">{avatarTile(nav)}</span>
        </button>
      </div>
    </header>
  );
});
