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
      return { title: "Downloading in Downpipe", message: activity.label, sticky: false };
    case "duplicate":
      return { title: "Already in Downpipe", message: activity.label, sticky: false };
    case "kept":
      return {
        title: "Downloaded by the browser",
        message: `${activity.label}\n${activity.message}`,
        sticky: true,
      };
    case "failed":
      return {
        title: "Not sent to Downpipe",
        message: `${activity.label}\n${activity.message}`,
        sticky: true,
      };
  }
}
