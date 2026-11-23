import { memo, type ComponentType } from "react";
import { ArrowLeft, Settings } from "lucide-react";
import { MobileNotifications } from "./MobileNotifications";
import { MobileProfile } from "./MobileProfile";
import { MobileSettings } from "./MobileSettings";
import { MobileSmartMixes } from "./MobileSmartMixes";
import { useMobileNav, type MobilePage } from "./mobileNav";
import type { MobileScreenProps } from "./types";

type BarPage = Exclude<MobilePage, "lyrics">;

/** Each page and the label beside its back arrow, as in Figma (F4/F5 BACK, F1 PROFILE, D5 LIBRARY). */
const PAGES: Record<BarPage, { label: string; Body: ComponentType<MobileScreenProps> }> = {
  settings: { label: "BACK", Body: MobileSettings },
  notifications: { label: "BACK", Body: MobileNotifications },
  profile: { label: "PROFILE", Body: MobileProfile },
  smart_mixes: { label: "LIBRARY", Body: MobileSmartMixes },
};

/** A page over the tabs: back bar (Profile adds Settings), then the page. Lyrics is a full sheet instead. */
export const MobilePageView = memo(function MobilePageView({ c, page }: MobileScreenProps & { page: BarPage }) {
  const nav = useMobileNav();
  const { label, Body } = PAGES[page];
  return (
    <div className="m-screen__container m-page" data-page={page}>
      <div className="m-backbar">
        <button type="button" className="m-backbar__back" onClick={nav.back} aria-label="Back">
          <span className="m-backbar__btn"><ArrowLeft size={20} strokeWidth={2} /></span>
          <span className="m-kicker m-kicker--ink">{label}</span>
        </button>
        {page === "profile" && (
          <button type="button" className="m-backbar__btn" onClick={() => nav.openPage("settings")} aria-label="Settings">
            <Settings size={20} strokeWidth={2} />
          </button>
        )}
      </div>
      <div className="m-page__body"><Body c={c} /></div>
    </div>
  );
});
