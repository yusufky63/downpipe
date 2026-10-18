// Words used across the app. {placeholders} are filled in at run time.
export const common = {
  appName: "Downpipe",
  cancel: "Cancel",
  save: "Save",
  close: "Close",
  browse: "Browse",
  dismiss: "Dismiss",
  keep: "Keep",
  open: "Open",
  pause: "Pause",
  resume: "Resume",
  retry: "Retry",
  remove: "Remove",
  showInFolder: "Show in folder",
  moreActions: "More actions",
  unknown: "Unknown",
  none: "–",
  /** A video page before its title is known, e.g. "YouTube video". */
  siteVideo: "{site} video",
  downloads: { one: "{count} download", other: "{count} downloads" },
  links: { one: "{count} link", other: "{count} links" },
};

export const states = {
  confirm: "Confirm",
  queued: "Queued",
  starting: "Connecting",
  downloading: "Downloading",
  waiting: "Retrying",
  verifying: "Verifying",
  paused: "Paused",
  completed: "Done",
  failed: "Failed",
  canceled: "Canceled",
  filters: {
    all: "All",
    active: "Active",
    queued: "Queued",
    completed: "Completed",
    failed: "Failed",
  },
};
