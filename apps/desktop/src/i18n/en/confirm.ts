// The confirmation window (IDM's "Download File Info") and a download's
// progress window. File names, URLs, paths and engine messages are filled in
// as they are; tool names (yt-dlp, FFmpeg) and stream kinds (HLS, DASH) stay
// the same in every language.
export const confirm = {
  /** A stream before its quality is known, e.g. "HLS stream". */
  stream: "{kind} stream",
  /** The eyebrow of a download added in Downpipe itself. */
  newDownload: "New download",
  fromBrowser: "From the browser",
  /** {what} is a site ("YouTube") or a stream ("HLS stream"). */
  fromBrowserWhat: "From the browser · {what}",
  /** {from} is one of the eyebrows above. */
  moreWaiting: { one: "{from} · {count} more waiting", other: "{from} · {count} more waiting" },
  titleVideo: "Download this video?",
  titleFile: "Download this file?",
  stopAsking: "Don't ask again",
  stopAskingTitle: "Downloads from the browser then start at once. Settings, App turns the question back on.",
  later: "Download later",
  start: "Start download",
  url: "URL",
  name: "Name",
  /** Hints on the name field: who decides the file's extension. */
  nameHint: {
    site: "yt-dlp decides the extension: .mp4 in most cases, .mkv or .webm for some formats, .m4a for audio only.",
    stream: "The stream decides the extension: .mp4 in most cases, .ts or .webm for some.",
    file: "The server may suggest another name; this one is kept.",
  },
  saveTo: "Save to",
  quality: "Quality",
  queue: "Queue",
  audio: "Audio",
  /** The quality list before it is read, or after reading it failed. */
  qualityBest: "Best",
  askingYtDlp: "Asking yt-dlp…",
  readingStream: "Reading the stream…",
  openMediaSettings: "Open Settings, Media",
  /** {message} is the engine's reason, ending with a full stop. */
  readFailedPage: "Could not read the page: {message} It is downloaded in the best quality when it starts.",
  readFailedStream: "Could not read the stream: {message} It is downloaded in the best quality when it starts.",
  noFfmpegStream: "No FFmpeg found, so video and sound are saved as two files.",
  setUpFfmpeg: "Set up FFmpeg",
  /** {link} becomes a button that reads `setUpFfmpegLink`. */
  noFfmpegSite: "This quality needs FFmpeg to join video and sound, and none was found. Pick Audio only or an H.264 + AAC quality, or {link}.",
  setUpFfmpegLink: "set up FFmpeg",
  /** An error without a message of its own, anywhere in the app. */
  unknownError: "Something went wrong",

  /** A download's progress window (IDM's download dialog). */
  progress: {
    /** The title while the download's name is not known. */
    download: "Download",
    eyebrow: {
      queued: "Queued",
      downloading: "Downloading",
      paused: "Paused",
      done: "Download complete",
      failed: "Download failed",
      canceled: "Canceled",
      removed: "Removed",
    },
    /** IDM's status line. */
    status: {
      queued: "Waiting for a free place in the queue",
      connecting: "Connecting",
      receiving: "Receiving data",
      /** Chosen by the number of fragments; {done} of them are downloaded. */
      fragments: { one: "Receiving data · {done} of {count} fragment", other: "Receiving data · {done} of {count} fragments" },
      connections: { one: "Receiving data · {count} connection", other: "Receiving data · {count} connections" },
      waiting: "Connection lost, trying again",
      verifying: "Checking the file",
      paused: "Paused",
      finished: "Finished",
      failed: "Failed",
      canceled: "Canceled; what was downloaded is deleted",
    },
    /** {approx} is "~" while a stream's size is an estimate, else empty. */
    size: "{downloaded} of {approx}{total} · {percent}%",
    /** {time} is like "42s" or "3:07". */
    timeLeft: "{speed} · {time} left",
    resumeYes: "Yes",
    resumeNo: "No: pausing starts it over",
    /** Row labels. "Resume" is whether the download can be resumed. */
    labels: {
      url: "URL",
      status: "Status",
      size: "Size",
      speed: "Speed",
      resume: "Resume",
      savedTo: "Saved to",
    },
    hide: "Hide",
    hideTitle: "Closes this window; the download goes on.",
    openFolder: "Open folder",
    cancelTitle: "Stops the download and deletes what was downloaded.",
    startAgain: "Start again",
    gone: "This download is no longer in the list.",
    /** The window's title, which is all the taskbar shows of a download. */
    title: {
      queued: "Queued · {name}",
      percent: "{percent}% {name}",
      paused: "Paused · {name}",
      done: "Done · {name}",
      failed: "Failed · {name}",
      canceled: "Canceled · {name}",
      removed: "Removed",
      /** Every title ends with the app's name. */
      window: "{title} - Downpipe",
    },
  },
};
