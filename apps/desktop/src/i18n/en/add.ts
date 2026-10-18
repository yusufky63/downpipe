// The dialog for adding links. {placeholders} are filled in at run time.
export const add = {
  eyebrow: "New download",
  title: "Add a link",
  /** Only shown for two links or more; a single link keeps `title`. */
  titleMany: { one: "Add {count} link", other: "Add {count} links" },
  addPaused: "Add paused",
  submit: {
    /** Every link is a video page: its quality is chosen in the next dialog. */
    next: "Next",
    addToList: "Add to list",
    download: "Download",
    downloadMany: { one: "Download {count} file", other: "Download {count} files" },
  },
  links: {
    label: "Links",
    /** The pattern example stays as written: it is the syntax the field accepts. */
    hint: "One link per line, or a pattern like file[001-100].zip. http, https and magnet links are supported.",
    found: { one: "{count} link found. One download each.", other: "{count} links found. One download each." },
    fromPattern: { one: "{count} link from the pattern. One download each.", other: "{count} links from the pattern. One download each." },
    patternCapped: { one: "Only the first {count} link of the pattern is used.", other: "Only the first {count} links of the pattern are used." },
    /** A single video page, e.g. "A YouTube video." */
    video: "A {site} video. You choose its quality next.",
    /** Added after the hint above when every link is a video page. */
    allVideos: "You choose the quality of each video next.",
    /** Added after the hint above when only some links are video pages. */
    someVideos: { one: "One of them is a video: you choose its quality next.", other: "{count} of them are videos: you choose their quality next." },
    importList: "Import a list",
    importHint: "A .txt file with one link per line. You can also drop it here.",
  },
  saveTo: {
    label: "Save to",
    automatic: "Automatic: the folder of the file's category.",
  },
  fileName: {
    label: "File name",
    placeholder: "Automatic",
    hint: "Leave empty to use the name the server sends.",
    videoHint: "Leave empty to use the video's title.",
    singleOnly: "Only for a single link.",
  },
  connections: {
    label: "Connections",
    hint: "Upper limit",
    videoHint: "Not for videos",
  },
  queue: {
    label: "Queue",
    hint: "Each queue runs its own downloads at once.",
  },
  checksum: {
    label: "Checksum",
    /** The sha256 example stays as written: it is the format the field accepts. */
    hint: "Optional, e.g. sha256:9f86d0… The file is only renamed into place if it matches.",
    videoHint: "Only for files: yt-dlp puts a video's file together from the site's formats.",
  },
  errors: {
    listTooLarge: "That file is too large for a list of links.",
    noLinks: "Paste at least one http, https or magnet link.",
    /** One line per failed link when several were added. */
    forLink: "{url}: {error}",
  },
};
