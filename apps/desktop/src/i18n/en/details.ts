// The details dialog: progress, live numbers, the new-link form and the file's facts.
export const details = {
  eyebrow: "Download details",
  openFile: "Open file",
  remove: "Remove…",
  cancelAndDelete: "Cancel and delete",
  cancelAndDeleteTitle: "Stops the download and deletes the partial file",
  /** A canceled download started over. */
  downloadAgain: "Download again",
  /** A finished file fetched again; opens a dialog first. */
  redownload: "Download again…",
  progress: "Progress",
  /** Bytes so far against the size, e.g. "12 MB of 40 MB". */
  progressOf: "{downloaded} of {total}",
  /** The same when the size is the site's estimate. */
  progressOfAbout: "{downloaded} of about {total}",
  /** The key under the segment bar. */
  legend: {
    downloaded: "Downloaded",
    connection: "Connection",
    remaining: "Remaining",
  },
  stats: {
    speed: "Speed",
    timeLeft: "Time left",
    connections: "Connections",
    /** How many connections the engine is working towards. */
    target: "target {count}",
    resume: "Resume",
    supported: "Supported",
    notSupported: "Not supported",
    size: "Size",
    downloaded: "Downloaded",
  },
  rows: {
    savedTo: "Saved to",
    folder: "Folder",
    source: "Source",
    video: "Video",
    stream: "Stream",
    bestQuality: "Best quality",
    fragments: { one: "{done} of {count} fragments", other: "{done} of {count} fragments" },
    queue: "Queue",
    /** The site rule a download got; its pattern follows. */
    siteRule: "Site rule",
    checksum: "Checksum",
    added: "Added",
    finished: "Finished",
  },
  /** Swapping an expired link for a fresh one. */
  relink: {
    button: "Use a new link…",
    label: "New link for this file",
    submit: "Continue",
    hint: "For an expired or broken link. The download continues where it stopped only if the new link serves the same file; otherwise nothing is overwritten.",
  },
  copy: "Copy",
  copied: "Copied",
};
