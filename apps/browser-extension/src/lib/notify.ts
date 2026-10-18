import { t } from "../i18n";
import type { Activity } from "./messages";

export interface NotificationText {
  title: string;
  message: string;
  /** Successes clear themselves; problems stay until dismissed. */
  sticky: boolean;
}

/** What the system notification says after a download was handed over, or not. */
export function notificationFor(activity: Activity): NotificationText {
  switch (activity.outcome) {
    case "added":
      // Not sticky either way: the app brings its question to the front, and
      // a notification left behind would outlive the answer.
      return activity.awaitingConfirmation
        ? {
            title: t("notifications.waitingTitle"),
            message: `${activity.label}\n${t("notifications.waitingBody")}`,
            sticky: false,
          }
        : { title: t("notifications.downloading"), message: activity.label, sticky: false };
    case "duplicate":
      return { title: t("notifications.duplicate"), message: activity.label, sticky: false };
    case "kept":
      return {
        title: t("notifications.kept"),
        message: `${activity.label}\n${activity.message}`,
        sticky: true,
      };
    case "failed":
      return {
        title: t("notifications.failed"),
        message: `${activity.label}\n${activity.message}`,
        sticky: true,
      };
  }
}
