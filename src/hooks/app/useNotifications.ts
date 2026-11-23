import { useCallback, useState } from "react";
import type { AppNotification, ToastAction } from "../../types";

const TOAST_MS = 3000;

/** In-app notifications (the inbox) and their short-lived toasts. */
export function useNotifications() {
  const [notifications, setNotifications] = useState<AppNotification[]>([]);
  const [toasts, setToasts] = useState<AppNotification[]>([]);

  const addAppNotification = useCallback(
    (type: "success" | "error" | "info" | "warning", title: string, message: string, action?: ToastAction) => {
      const notification: AppNotification = {
        id: "notif_" + Date.now() + "_" + Math.random().toString(36).substring(2, 7),
        type,
        title,
        message,
        timestamp: Date.now(),
        read: false,
        action,
      };
      setNotifications((prev) => [notification, ...prev]);
      setToasts((prev) => [...prev, notification]);
      setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== notification.id));
      }, TOAST_MS);
    },
    [],
  );
  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);
  const markNotificationAsRead = useCallback((id: string) => {
    setNotifications((prev) => prev.map((n) => (n.id === id ? { ...n, read: true } : n)));
  }, []);
  const markAllNotificationsAsRead = useCallback(() => {
    setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
  }, []);
  const clearAllNotifications = useCallback(() => setNotifications([]), []);
  const deleteNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  return {
    notifications, toasts, unreadNotificationsCount: notifications.filter((n) => !n.read).length,
    addAppNotification, dismissToast, markNotificationAsRead, markAllNotificationsAsRead,
    clearAllNotifications, deleteNotification,
  };
}
