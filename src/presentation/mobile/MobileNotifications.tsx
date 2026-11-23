import { memo } from "react";
import { Check, CheckCheck, Trash2, X } from "lucide-react";
import { WindowedList } from "../WindowedList";
import type { AppNotification } from "../../types";
import type { MobileScreenProps } from "./types";

const ROW_HEIGHT = 117;

/** "2M AGO", "1H AGO", "3D AGO". */
function ago(timestamp: number): string {
  const minutes = Math.max(0, Math.floor((Date.now() - timestamp) / 60000));
  if (minutes < 1) return "JUST NOW";
  if (minutes < 60) return `${minutes}M AGO`;
  const hours = Math.floor(minutes / 60);
  return hours < 24 ? `${hours}H AGO` : `${Math.floor(hours / 24)}D AGO`;
}

/**
 * Notifications (Figma F4): unread count, Mark All Read / Clear All, then one
 * row per notification (red dot while unread) with mark-read and remove.
 */
export const MobileNotifications = memo(function MobileNotifications({ c }: MobileScreenProps) {
  const items = c.notifications;
  const unread = items.filter((n) => !n.read).length;

  return (
    <div className="m-notes">
      <div className="m-notes__hero">
        <span className={`m-kicker ${unread ? "" : "m-kicker--muted"}`}>UNREAD {unread}</span>
        <h1 className="m-display m-notes__title">NOTIFICATIONS.</h1>
      </div>
      <div className="m-notes__actions">
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" disabled={!unread} onClick={c.markAllNotificationsAsRead}>
          <CheckCheck size={18} strokeWidth={2} /><span>MARK ALL READ</span>
        </button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" disabled={!items.length} onClick={c.clearAllNotifications}>
          <Trash2 size={18} strokeWidth={2} /><span>CLEAR ALL</span>
        </button>
      </div>
      <div className="m-notes__list">
        {items.length ? (
          <WindowedList
            items={items}
            rowHeight={ROW_HEIGHT}
            label="Notifications"
            fill
            renderItem={(n: AppNotification) => (
              <div key={n.id} className={`m-note ${n.read ? "" : "is-unread"} is-${n.type}`}>
                <span className="m-note__dot" aria-hidden="true" />
                <div className="m-note__text">
                  <strong className="m-note__title">{n.title}</strong>
                  <span className="m-note__message">{n.message}</span>
                  <span className="m-note__time">{ago(n.timestamp)}</span>
                </div>
                <div className="m-note__actions">
                  <button type="button" className="m-note__btn" disabled={n.read} onClick={() => c.markNotificationAsRead(n.id)} aria-label="Mark as read">
                    <Check size={18} strokeWidth={2} />
                  </button>
                  <button type="button" className="m-note__btn" onClick={() => c.deleteNotification(n.id)} aria-label="Remove">
                    <X size={18} strokeWidth={2} />
                  </button>
                </div>
              </div>
            )}
          />
        ) : (
          <p className="m-empty-msg">Nothing new. Downloads, scans and new mixes show up here.</p>
        )}
      </div>
    </div>
  );
});
