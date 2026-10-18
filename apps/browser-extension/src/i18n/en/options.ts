// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are.
export const options = {
  pageTitle: "Filters · Downpipe",
  headLabel: "Browser extension",
  lede:
    "Choose which downloads the browser hands to Downpipe while Capture browser downloads is on, and which media the extension lists while Media on this page is on. Everything else stays in the browser as usual.",
  captureOff:
    "Capture is off. These filters apply once you turn on Capture browser downloads in the Downpipe toolbar popup.",
  loadError: "The settings could not be loaded. Reload this page to try again.",
  typesHeading: "01 — File types",
  captureAll: "Capture all file types",
  captureAllDetail: "Every download the browser starts goes to Downpipe, whatever its type.",
  typesLabel: "Only these file types",
  typesHelp:
    "Extensions separated by spaces or commas, such as {example}. A download is captured when its file name ends with one of them; {tarGz} works too.",
  restoreDefault: "Restore the default list",
  sitesHeading: "02 — Excluded sites",
  sitesLabel: "Never capture or list media on these sites",
  sitesHelp:
    "One site per line. {wildcard} also covers every subdomain. A download stays in the browser when it comes from one of these sites or starts on one of their pages, and media they play is not listed. Ports and paths are ignored.",
  sizeHeading: "03 — Minimum size",
  sizeLabel: "Leave smaller files to the browser",
  sizeHelp: "0 turns this off. When the server does not say how big a file is, it goes to Downpipe.",
  mediaHeading: "04 — Media on pages",
  mediaHelp:
    "Used while Media on this page is on in the toolbar popup. HLS and DASH streams are always listed; the excluded sites above apply here too.",
  overlay: "Show a Download button on videos",
  overlayDetail: "On a playing video, for a few seconds and while the pointer is over it.",
  mediaTypesLabel: "List media files of these types",
  mediaTypesHelp:
    "Extensions separated by spaces or commas. A file without one is judged by the type the server sends. An empty list lists streams only.",
  mediaSizeLabel: "Leave out media files smaller than",
  mediaSizeHelp:
    "Keeps notification sounds and short previews out. 0 lists every size; files whose size the server does not say are always listed.",
  appHeading: "05 — Desktop app",
  startApp: "Start Downpipe when it is closed",
  startAppDetail:
    "A download sent from the browser starts Downpipe in the tray first, instead of failing. When this is off, Downpipe has to be open.",
  save: "Save",
  /** After the first few errors under a field. */
  moreErrors: { one: "And {count} more.", other: "And {count} more." },
  unsaved: "Unsaved changes",
  notSavedFix: "Not saved. Fix the fields marked in red.",
  saving: "Saving…",
  notSavedNoAnswer: "Not saved. The extension did not answer; try again.",
  saved: "Saved",
  /** Under the fields. {value} is what the user typed. */
  errors: {
    notExtension: "\"{value}\" is not a file extension.",
    tooManyTypes: { one: "List at most {count} file type.", other: "List at most {count} file types." },
    notSite: "Line {line}: \"{value}\" is not a site. Use a host such as example.com or *.example.com.",
    tooManySites: { one: "List at most {count} site.", other: "List at most {count} sites." },
    sizeNotNumber: "Enter a number such as 2 or 1.5, or 0 to turn this off.",
    sizeTooBig: "Enter a size under 1 TiB.",
    noTypes: "Add at least one file type, or turn on Capture all file types.",
  },
};
