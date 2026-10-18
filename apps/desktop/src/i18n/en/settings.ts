// The settings dialog, one group per tab, and the browser bridge's states.
// {placeholders} that hold an element (a path, a command) are put in by the dialog.
export const settings = {
  eyebrow: "Settings",
  title: "Preferences",
  tabsLabel: "Settings sections",
  tabs: {
    downloads: "Downloads",
    queues: "Queues",
    categories: "Categories",
    network: "Network",
    media: "Media",
    schedule: "Schedule",
    app: "App",
  },
  downloads: {
    folder: "Default folder",
    folderHint: "Categories below create their folders inside this one.",
    concurrent: "Downloads at once",
    concurrentHint: "Others wait in the queue. 1 to 16.",
    connections: "Connections per download",
    connectionsHint: "Upper limit. Downpipe starts with 2 and adds more only while they help. 1 to 32.",
    speedLimit: "Speed limit",
    speedLimitHint: "Shared by all downloads. Leave empty for no limit. Applies immediately.",
    noLimit: "No limit",
    unit: "Speed limit unit",
  },
  queues: {
    intro:
      "Each queue runs its own number of downloads at once, next to the others; inside a queue, downloads start top to bottom. Move downloads between queues and change their order from the right-click menu. Downloads in a removed queue move to the main queue.",
    label: "Queues",
    name: "Queue name",
    namePlaceholder: "Name",
    /** The main queue's limit, which is set on the Downloads tab. */
    mainLimit: { one: "{count} at once", other: "{count} at once" },
    mainLimitTitle: "Set in Downloads, Downloads at once",
    limit: "{name} downloads at once",
    limitUnnamed: "Queue downloads at once",
    limitTitle: "Downloads at once, 1 to 16",
    remove: "Remove {name}",
    removeUnnamed: "Remove queue",
    /** The name a new queue starts with, e.g. "Queue 2". */
    newName: "Queue {number}",
    add: "Add queue",
  },
  categories: {
    use: "Sort downloads into category folders",
    useHint:
      "A new download goes to its category's folder by file type. When the server's file name shows a different type, the finished file moves to the right folder. Folders you pick yourself are never changed.",
    label: "Categories",
    name: "Category name",
    namePlaceholder: "Name",
    types: "{name} file types",
    typesUnnamed: "Category file types",
    remove: "Remove {name}",
    removeUnnamed: "Remove category",
    folder: "{name} folder",
    folderUnnamed: "Category folder",
    folderPlaceholder: "Folder: {name} inside the default folder",
    folderPlaceholderUnnamed: "Folder: name inside the default folder",
    add: "Add category",
  },
  network: {
    proxy: "Proxy",
    proxyHint: "http://, https:// or socks5://host:port. Empty uses the system proxy. Used by downloads started after saving.",
    proxyPlaceholder: "System proxy",
    userAgent: "User agent",
    userAgentHint: "Some servers only answer browsers. Empty sends Downpipe's own.",
  },
  media: {
    ffmpegHint:
      "Merges the separate video and sound of a stream into one file, without re-encoding. Empty uses ffmpeg from PATH. Downpipe does not include FFmpeg.",
    ffmpegPlaceholder: "ffmpeg on PATH",
    found: "Found",
    notFound: "Not found",
    ffmpegMissing:
      "Streams whose video and sound come separately are saved as two files. Install FFmpeg and add it to PATH, or enter the path to ffmpeg above, then save.",
    streams:
      "Streams come from the browser extension's media detection, or from .m3u8 and .mpd links added here. DRM-protected and live streams are not downloaded.",
  },
  /** yt-dlp and Deno, on the Media tab. */
  siteTools: {
    heading: "Videos on pages",
    intro:
      "On a YouTube page, Download with Downpipe hands the page to yt-dlp, which picks the video out. yt-dlp needs Deno for YouTube and FFmpeg to join video and sound above 360p.",
    ytDlpHint: "Empty uses the copy downloaded below, else yt-dlp from PATH.",
    ytDlpPlaceholder: "Downloaded copy, else yt-dlp on PATH",
    denoHint: "Version 2.3.0 or newer. Empty uses the copy downloaded below, else deno from PATH, else Node 22 or newer.",
    denoPlaceholder: "Downloaded copy, else deno on PATH",
    looking: "Looking for yt-dlp and Deno…",
    listLabel: "Tools for videos on pages",
    downloading: "Downloading…",
    /** {folder} is the folder the tools go into. */
    installNote:
      "Only when you press it: the official builds from github.com/yt-dlp/yt-dlp and github.com/denoland/deno, each checked against the SHA-256 its release publishes, go into {folder}. YouTube changes often; press it again for the latest yt-dlp.",
    toolsFolder: "Downpipe's tools folder",
    cookies:
      "Downpipe sends yt-dlp no cookies, so private, members-only and age-restricted videos are not downloaded, nor are protected (DRM) and live ones. Following YouTube's terms is up to you.",
  },
  schedule: {
    enable: "Start and stop downloads on a schedule",
    enableHint:
      "At the start time, paused and failed downloads resume. At the stop time, they pause. Downpipe has to be running, in the window or the tray.",
    queue: "Queue",
    queueHint: "The other queues are not touched.",
    everyQueue: "Every queue",
    start: "Start at",
    stop: "Stop at",
    stopHint: "Optional",
    days: "Days",
  },
  app: {
    language: "Language",
    languageHint: "Applies at once, to the window, the tray menu and notifications.",
    systemDefault: "System default ({language})",
    notify: "Notify when a download finishes",
    notifyHint: "A system notification when a download completes or fails while the window is not in front.",
    tray: "Keep running in the tray when the window closes",
    trayHint: "Downloads continue, and the browser extension can still send links. Quit from the tray icon.",
    startup: "Start Downpipe when you sign in",
    startupHint: "Starts in the tray, so captured downloads always have somewhere to go.",
    clipboard: "Offer to download copied links",
    clipboardHint:
      "When you copy a link to a file type listed under Categories, or to a YouTube video, Downpipe opens the Add dialog with it. Links already in the list are skipped.",
    awake: "Keep the computer awake while downloading",
    awakeHint: "Sleep waits until downloads stop running. The screen can still turn off.",
    confirmBrowser: "Ask before downloads from the browser start",
    confirmBrowserHint:
      "Choose the file name, folder and queue first, in a small window over the browser. Off: they start at once with the automatic folder.",
  },
  bridge: {
    heading: "Browser bridge",
    register: "Register with browsers at start-up",
    registerHint:
      "Lets the Downpipe extension in Chrome, Edge, Chromium and Firefox reach this copy of the app. Changes your user account only. Turn it off to manage the registration with downpipe-native-host yourself.",
    checking: "Checking the registration…",
    /** {host} is the path where the bridge should be, {command} the command that builds it. */
    noHost: "This copy of Downpipe has no browser bridge at {host}. Reinstall Downpipe; in a development build, run {command}.",
    listLabel: "Registration per browser",
    registering: "Registering…",
    registerNow: "Register now",
    pickUp: "Open browsers pick it up the next time the extension checks.",
    /** One browser's registration, as a badge. */
    states: {
      registered: "Registered",
      notInstalled: "Not installed",
      missing: "Not registered",
      invalid: "Unreadable",
      otherHost: "Other copy",
      extensionNotAllowed: "Other extension",
      failed: "Error",
    },
  },
  backup: {
    heading: "Backup and import",
    intro:
      "Save the settings, queues, categories and the download list to one file, to bring them back here or on another computer. Headers and cookies are never saved.",
    export: "Export settings and list…",
    import: "Import…",
    reading: "Reading…",
    /** {file} is the backup's file name. */
    exported: {
      one: "Saved the settings and {count} download to {file}.",
      other: "Saved the settings and {count} downloads to {file}.",
    },
    /** Heading of the import preview; {file} is the backup's file name. */
    previewHeading: "Import {file}",
    madeBy: "Made by Downpipe {version} on {date}.",
    madeOn: "Made on {date}.",
    madeUnknown: "The file does not say when it was made.",
    settings: "Settings",
    /** {queues} and {categories} are numbers. */
    settingsHint:
      "Takes the file's settings, its queues ({queues}) and categories ({categories}). Queues and categories with the same name are updated; the ones only this computer has stay. The paths to FFmpeg, yt-dlp and Deno stay as they are.",
    downloads: "Downloads",
    /** {count} new of {total} in the file. */
    downloadsHint: {
      one: "{count} of {total} is new. Unfinished downloads are added paused, links already in the list are skipped, and no file is replaced.",
      other: "{count} of {total} are new. Unfinished downloads are added paused, links already in the list are skipped, and no file is replaced.",
    },
    invalid: {
      one: "{count} entry in the file is damaged and will be left out.",
      other: "{count} entries in the file are damaged and will be left out.",
    },
    importNow: "Import",
    result: {
      settings: "The settings were imported.",
      added: { one: "Added {count} download.", other: "Added {count} downloads." },
      duplicates: { one: "{count} was already in the list.", other: "{count} were already in the list." },
      invalid: { one: "{count} damaged entry was left out.", other: "{count} damaged entries were left out." },
    },
    errors: {
      notABackup: "This file is not a Downpipe backup.",
      tooNew: "This backup was made by a newer Downpipe. Update Downpipe to import it.",
      damaged: "This backup is damaged and cannot be imported.",
      tooLarge: "This file is too large to import.",
    },
  },
  idm: {
    heading: "Import from Internet Download Manager",
    intro: "Brings IDM's downloads into the list, paused. IDM's own list and files are only read, never changed.",
    fromComputer: "Read IDM's list on this computer",
    fromFile: "Open an exported list…",
    fileHint:
      "In IDM: Tasks, Export, To text file or To IDM export file (.ef2). Only links and file names are read; cookies and headers in the file are skipped.",
    sourceComputer: "IDM's list on this computer",
    /** {file} is the exported list's file name. */
    sourceFile: "From {file}",
    empty: "No downloads found.",
    listLabel: "Downloads in IDM's list",
    selectAll: "Select all",
    selectNone: "Select none",
    finished: "Finished in IDM",
    listed: "In Downpipe",
    /** In place of a folder for links from an exported list. */
    defaultFolder: "Folder by category",
    sizeUnknown: "Size unknown",
    add: { one: "Add {count} paused", other: "Add {count} paused" },
    unreadable: "IDM's list could not be read.",
  },
};
