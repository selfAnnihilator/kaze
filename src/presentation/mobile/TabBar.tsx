import { memo, Fragment } from "react";
import type { MobileTab } from "./tabs";

interface TabItem {
  id: MobileTab;
  num: string;
  label: string;
}

const TABS: readonly TabItem[] = [
  { id: "home", num: "01", label: "HOME" },
  { id: "discover", num: "02", label: "DISCOVER" },
  { id: "library", num: "03", label: "LIBRARY" },
  { id: "queue", num: "04", label: "QUEUE" },
] as const;

/**
 * Brutalist bottom navigation (Figma 1:7799):
 * 60px height, 2px line border, 4 segments with 2px dividers,
 * 4px active top bar, Courier Prime index, Inter Extra Bold label.
 */
export const TabBar = memo(function TabBar({
  active, onSelect,
}: { active: MobileTab; onSelect: (tab: MobileTab) => void }) {
  return (
    <nav className="m-tabbar" aria-label="Primary navigation">
      <div className="m-tabbar__border" />
      <div className="m-tabbar__row">
        {TABS.map((t, idx) => {
          const isActive = active === t.id;
          return (
            <Fragment key={t.id}>
              {idx > 0 && <div className="m-tabbar__divider" />}
              <button
                type="button"
                className={`m-tabbar__item ${isActive ? "is-active" : ""}`}
                aria-current={isActive ? "page" : undefined}
                onClick={() => onSelect(t.id)}
              >
                {isActive && <div className="m-tabbar__indicator" />}
                <div className="m-tabbar__content">
                  <span className="m-tabbar__num">{t.num}</span>
                  <span className="m-tabbar__label">{t.label}</span>
                </div>
              </button>
            </Fragment>
          );
        })}
      </div>
    </nav>
  );
});
