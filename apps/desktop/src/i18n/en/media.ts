// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
export const media = {
  subtitles: {
    label: "Subtitles",
    none: "None",
    uploaded: "From the uploader",
    auto: "Automatic captions",
    original: "{language} (original)",
    embedded: "The subtitles go into the video file.",
    separate: "The subtitles are saved next to the video as a file.",
  },
  siteErrors: {
    forbidden: "YouTube refused the download. A newer yt-dlp usually fixes this: update it in Settings, Media, then try again.",
    signIn: "YouTube wants a signed-in browser for this download. Downpipe does not use browser cookies: wait a while or use another network, then try again.",
    ageRestricted: "YouTube shows this video only to signed-in adults. Downpipe does not use accounts or browser cookies, so it cannot download it.",
    unavailable: "This video is unavailable: it is private, removed, members-only or blocked in your region.",
    jsRuntime: "yt-dlp could not answer YouTube's JavaScript challenges. Download Deno in Settings, Media, then try again.",
    poToken: "YouTube asked for a proof-of-origin (PO) token for this video. Update yt-dlp in Settings, Media, or choose another quality.",
    outdated: "yt-dlp no longer understands this page; YouTube probably changed it. Update yt-dlp in Settings, Media, then try again.",
  },
  /** A quality with sound and no picture; also what the engine calls it. */
  audioOnly: "Audio only",
  /** A quality the manifest gives neither a height nor a bit rate. */
  defaultQuality: "Default",
  /** The estimated size in a quality's details, e.g. "about 10.0 MiB". */
  aboutSize: "about {size}",
  /** An audio track with neither a name nor a language. */
  audio: "Audio",
  tools: {
    found: "Found",
    notFound: "Not found",
    problem: "Problem",
    /** After the tool's path, e.g. "version 2026.08.19". */
    version: "version {version}",
    /** Where the tool was found, after its path and version. */
    sources: {
      settings: "set in Settings",
      managed: "downloaded by Downpipe",
      path: "on PATH",
    },
    /** Why a tool that is not found matters. */
    missing: {
      ytDlp: "Needed for videos on pages.",
      runtime: "Without Deno (or Node) yt-dlp misses some YouTube qualities.",
      ffmpeg: "Without it, only “Audio only” and qualities with their own sound work.",
      /** Where Downpipe has no FFmpeg build to download (macOS). */
      ffmpegManual: "Downpipe cannot download FFmpeg for this system. Install it (on macOS: brew install ffmpeg) and enter its path at the top of this tab, such as /opt/homebrew/bin/ffmpeg.",
    },
    /** The button on a tool's row in Settings, Media. */
    download: "Download",
    update: "Update",
    /** Downloads every missing tool; {size} is like "231 MiB". */
    installAll: "Install all (about {size})",
    /** The one button a dialog shows when a download needs a missing tool. */
    offer: "Download the missing tools (about {size})",
    /** {tools} is a list such as "yt-dlp, Deno and FFmpeg". */
    missingForVideo: "Missing for this video: {tools}.",
    missingForQuality: "Missing for this quality: {tools}.",
    starting: "Starting the download…",
    /** {done} is a number of MiB, {total} like "163 MiB". */
    progress: "Downloading {tool}… {done} of {total}",
    progressUnknown: "Downloading {tool}… {done}",
    /** After the download: unpacking and putting the tool in place. */
    installing: "Installing {tool}…",
    /** The offer button's tooltip. */
    sourceNote: "The official releases on GitHub, each checked against the SHA-256 its release publishes. Nothing is downloaded before you press it.",
    /** {time} is a date and time. */
    checked: "Last looked for a newer version: {time}",
    neverChecked: "Not looked for a newer version yet. Downpipe looks once a day, when a video download starts.",
    autoUpdate: "Keep yt-dlp up to date",
    autoUpdateHint:
      "Once a day, when a video download starts, Downpipe looks for a newer yt-dlp and installs it, checked against its SHA-256 like a download from here. Only the copy Downpipe downloaded is replaced, never one of your own.",
  },
};
