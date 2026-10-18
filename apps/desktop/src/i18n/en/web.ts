// Finding files on the web: the site grabber, feed subscriptions and file hosts.
export const grabber = {
  /** The header button that opens the grabber. */
  open: "Site grabber",
  eyebrow: "Site grabber",
  title: "Find files on a site",
  address: {
    label: "Start address",
    hint: "Downpipe reads this page and follows its links. No cookies are sent, so it sees what a visitor who is not signed in sees.",
  },
  depth: {
    label: "Depth",
    hint: "1 reads only this page; each step follows one more level of links.",
  },
  pages: {
    label: "Page limit",
    hint: "Pages read at most",
  },
  types: {
    label: "File types",
    custom: "Other types",
    customPlaceholder: "e.g. iso, torrent",
  },
  sameSite: {
    label: "Stay on this site",
    hint: "Only pages of the start address's site are read. Files linked from other sites are still listed.",
  },
  robots: {
    label: "Respect robots.txt",
    hint: "Skip the pages a site asks crawlers to leave alone.",
  },
  sizes: {
    label: "Look up file sizes",
    hint: "One small request per file, for the first {count} files.",
  },
  delay: {
    label: "Delay",
    hint: "Milliseconds between two requests to the same site. Sites may block visitors that hurry.",
  },
  start: "Find files",
  stop: "Stop",
  again: "New search",
  progress: {
    pages: { one: "{done} of {count} page read", other: "{done} of {count} pages read" },
    sizes: { one: "Looking up sizes: {done} of {count} file", other: "Looking up sizes: {done} of {count} files" },
    files: { one: "{count} file found", other: "{count} files found" },
  },
  results: {
    title: { one: "{count} file found", other: "{count} files found" },
    none: "No files of these types were found.",
    canceled: "Stopped early: these are the files found until then.",
    pageLimit: "The page limit was reached before every page was read.",
    blocked: { one: "{count} page or file was skipped because robots.txt disallows it.", other: "{count} pages or files were skipped because robots.txt disallows them." },
    failed: { one: "{count} page could not be read.", other: "{count} pages could not be read." },
    filter: "Filter files",
    selectAll: "Select all shown files",
    name: "Name",
    size: "Size",
    foundOn: "Found on",
    selected: { one: "{count} selected, {size}", other: "{count} selected, {size}" },
    unknownSizes: { one: "{count} of unknown size", other: "{count} of unknown size" },
    add: { one: "Add {count} file", other: "Add {count} files" },
    added: { one: "{count} file added.", other: "{count} files added." },
    alreadyListed: { one: "{count} was already in the list.", other: "{count} were already in the list." },
  },
  errors: {
    invalid_url: "Enter an http or https address.",
    no_types: "Choose at least one file type.",
    robots_blocked: "This site's robots.txt asks crawlers not to read this page. Turn off Respect robots.txt to read it anyway.",
    start_page: "The start page could not be read: {error}",
    not_html: "The start address is not a web page. Add it as a download instead.",
  },
};

export const feeds = {
  /** The header button that opens the feeds. */
  open: "Feeds",
  eyebrow: "Feeds",
  title: "RSS and Atom feeds",
  intro: "Downpipe checks each feed on its interval and adds the files of new items to the queue you choose. An item is added once, even after you remove its download.",
  empty: "No feeds yet. Add a podcast, a release feed or any other RSS or Atom address.",
  add: "Add a feed",
  checkNow: "Check now",
  edit: "Edit",
  off: "Off",
  checked: "Checked {date}",
  notChecked: "Not checked yet",
  error: "Last check failed: {error}",
  addedTotal: { one: "{count} download added in all", other: "{count} downloads added in all" },
  checkResult: {
    none: "No new items in {name}.",
    added: { one: "{count} download added from {name}.", other: "{count} downloads added from {name}." },
  },
  confirmRemove: "Remove {name}? The downloads it added stay in the list.",
  form: {
    newTitle: "New feed",
    editTitle: "Edit feed",
    url: "Feed address",
    types: "File types",
    typesHint: "Empty: any file the items offer. For example mp3, m4a.",
    pattern: "Title filter",
    patternHint: "A regular expression the title must contain, case ignored, for example episode|bonus. Empty: any title.",
    interval: "Check every",
    queue: "Queue",
    enabled: "Check this feed",
    enabledHint: "Off: the feed stays in the list but is not checked.",
    existing: "Download what the feed holds now",
    existingHint: "Off: only items published from now on are downloaded.",
    save: "Save",
    subscribe: "Add feed",
    subscribed: { one: "Feed added. {count} download added.", other: "Feed added. {count} downloads added." },
    subscribedNone: "Feed added. New items will be downloaded from now on.",
  },
  interval: {
    minutes: { one: "{count} minute", other: "{count} minutes" },
    hours: { one: "{count} hour", other: "{count} hours" },
    days: { one: "{count} day", other: "{count} days" },
  },
  errors: {
    duplicate_feed: "This feed is already in the list.",
    not_a_feed: "That address is a web page, not an RSS or Atom feed.",
    invalid_url: "Enter an http or https address.",
  },
};

export const hosts = {
  /** Shown for a file host's file that is not shared publicly (Google Drive). */
  privateFile: "The file is not shared publicly. Ask its owner to share it with anyone who has the link, or download it in your browser while signed in.",
};
