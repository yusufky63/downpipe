// Words used on several screens. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
export const common = {
  download: "Download",
  sending: "Sending…",
  checkAgain: "Check again",
  setupGuide: "Setup guide",
  filters: "Filters",
  /** The KiB / MiB picker next to a size field. */
  unit: "Unit",
  extensionNoAnswer: "The extension did not answer.",
  /** When a download in the popup's list was sent. */
  justNow: "just now",
  minutesAgo: { one: "{count} min ago", other: "{count} min ago" },
  hoursAgo: { one: "{count} h ago", other: "{count} h ago" },
};

/** What happened to a download handed to the app, in the popup, badge and overlay. */
export const results = {
  added: "Added to Downpipe",
  /** The app took it but asks for a name and folder before it starts. */
  waiting: "Waiting for your answer in Downpipe",
  /** The overlay's and media list's short form of `added`. */
  sent: "Sent to Downpipe",
};
