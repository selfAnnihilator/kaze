import { memo } from "react";
import { Compass, Home, Library, ListMusic } from "lucide-react";
import { MOBILE_TABS, type MobileTab } from "./tabs";

const ICONS = { home: Home, discover: Compass, library: Library, queue: ListMusic } as const;

/** Bottom navigation: Home, Discover, Library, Queue. */
export const TabBar = memo(function TabBar({
  active, onSelect,
}: { active: MobileTab; onSelect: (tab: MobileTab) => void }) {
  return (
    <nav className="m-tabbar" aria-label="Primary">
      {MOBILE_TABS.map(({ id, label }) => {
        const Icon = ICONS[id];
        return (
          <button key={id} type="button" className={`m-tabbar__item ${active === id ? "is-active" : ""}`}
            aria-current={active === id ? "page" : undefined} onClick={() => onSelect(id)}>
            <Icon aria-hidden="true" size={20} />
            <span>{label}</span>
          </button>
        );
      })}
    </nav>
  );
});
