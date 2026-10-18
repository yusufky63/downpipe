// The toolbar popup.
export const popup = {
  /** A word in the header while the app is connected. */
  connected: "Connected",
  /** Its tooltip: `status.connected` and `status.connectedDetail`. */
  connectedTitle: "{title}. {detail}",
  /** Label of the connection section, for screen readers. */
  desktopApp: "Desktop app",
  mediaHeading: "Media on this page",
  detectMedia: "Detect media on pages",
  downloadsHeading: "Downloads",
  pageLinks: "Links on this page…",
  emptyActivity: "Right-click a link and choose Download with Downpipe.",
  capture: "Capture browser downloads",
  captureDetail: "The browser keeps them when Downpipe is closed.",
  notifications: "Notifications",
  notificationsDetail: "When a download is handed over, or not.",
  /** Short so the row stays on one line; the host is in the tooltip. */
  pauseSite: "Pause capture on this site",
  resumeSite: "Resume capture on this site",
  pauseSiteTitle: "Downloads from {host}, or started on its pages, stay in the browser",
  resumeSiteTitle: "Downloads from {host} go to Downpipe again",
  /** {site} is the excluded-site pattern that covers this site, e.g. *.example.com. */
  pausedBy: "Paused here by {site}",
  progress: "{label} progress",
  downloadItem: "Download {label} with Downpipe",
  /** Badges for a download's live state in the app. */
  state: {
    queued: "Queued",
    starting: "Starting",
    downloading: "Downloading",
    waiting: "Waiting",
    verifying: "Verifying",
    paused: "Paused",
    completed: "Done",
    failed: "Failed",
    canceled: "Canceled",
  },
  /** Badges for what happened when it was sent, when the app cannot be asked. */
  outcome: {
    added: "Sent",
    duplicate: "In Downpipe",
    failed: "Not sent",
    kept: "In browser",
  },
  /** Held in the app's confirmation dialog. The detail is short: the badge leaves it little room. */
  waitingBadge: "Waiting in Downpipe",
  needsAnswer: "Needs your answer",
  downloadFailed: "The download failed",
  canceledInApp: "Canceled in Downpipe",
};
