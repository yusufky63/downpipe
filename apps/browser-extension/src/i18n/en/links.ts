// The links page: links collected from a page, to pick and send.
export const links = {
  pageTitle: "Links · Downpipe",
  headLabel: "Links on a page",
  title: "Links on this page",
  titleHost: "Links on {host}",
  found: { one: "{count} link found.", other: "{count} links found." },
  /** The page had more links than the list keeps; {max} is that limit. */
  foundTruncated: {
    one: "{count} link found. Only the first {max} are listed.",
    other: "{count} links found. Only the first {max} are listed.",
  },
  /** Label of the list section, for screen readers. */
  pickerLabel: "Links",
  filterPlaceholder: "Filter by text or address",
  filterLabel: "Filter links by text or address",
  hidePages: "Hide web pages",
  fileTypes: "File types",
  allTypes: "All types",
  /** The chip for the less common file types. */
  otherTypes: "Other",
  selectAll: "Select all",
  selectNone: "Select none",
  selected: "{chosen} of {total} selected",
  emptyOnlyPages: "Every link here is a web page. Turn off Hide web pages to see them.",
  emptyNoMatch: "No links match the filter.",
  stop: "Stop",
  downloadCount: { one: "Download {count} link", other: "Download {count} links" },
  checkingApp: "Checking Downpipe…",
  sendingProgress: "Sending {done} of {total}…",
  /** {reason} is the connection's state, e.g. "Downpipe is not running". */
  nothingSentBecause: "Nothing sent: {reason}.",
  stoppedBecause: "Stopped: {reason}",
  stopped: "Stopped. {summary}",
  /** The summary after sending: the parts joined by `separator`, then `summary`. */
  added: { one: "{count} added", other: "{count} added" },
  duplicate: { one: "{count} already in Downpipe", other: "{count} already in Downpipe" },
  failed: { one: "{count} failed", other: "{count} failed" },
  separator: ", ",
  summary: "{parts}.",
  nothingSent: "Nothing sent.",
  /** Badges on each row after it was sent. */
  row: {
    sending: "Sending",
    added: "Added",
    duplicate: "In Downpipe",
    failed: "Failed",
  },
  gone: "These links are no longer available. Go back to the page and choose Download links on this page with Downpipe again.",
  unreadable:
    "The browser does not let extensions read this page. Its own pages, extension stores and some built-in viewers cannot be read.",
  noLinks: "No http or https links were found on this page.",
};
