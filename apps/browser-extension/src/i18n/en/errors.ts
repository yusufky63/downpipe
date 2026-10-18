// Problems the extension words itself. Messages the app sends over the
// browser bridge for codes not listed here are shown as the app wrote them.
export const errors = {
  /** The browser bridge itself failed, before the app had a say. */
  bridge: {
    host_missing: "The Downpipe browser bridge is not installed.",
    host_forbidden: "The browser bridge is installed but does not allow this extension.",
    host_error: "The browser bridge stopped unexpectedly.",
    timeout: "Downpipe did not answer in time.",
    bad_response: "The browser bridge sent an answer this extension does not understand.",
  },
  /** Error codes from the bridge and the app's download queue. */
  code: {
    app_not_running: "Downpipe is not running. Open the Downpipe app and try again.",
    app_unavailable: "Downpipe did not answer. Restart the app and try again.",
    duplicate: "Already in Downpipe.",
    invalid_url: "This link cannot be downloaded. Only http and https links work.",
    version_mismatch:
      "Downpipe and its browser bridge are different versions. Restart Downpipe so it registers its own bridge.",
    unsupported_version: "This extension is newer than the installed Downpipe. Update Downpipe.",
    message_too_large: "The link is too long to hand over.",
    unsupported_page: "Downpipe does not download videos from this site.",
  },
  /** An unknown code that came without a message. */
  refused: "Downpipe refused the download ({code}).",
  streamsUnsupported: "This version of Downpipe cannot download streams. Update Downpipe.",
  pagesUnsupported: "This version of Downpipe cannot download videos from pages. Update Downpipe.",
  /** A captured download the browser kept. */
  keptClosed: "Downpipe was not running, so the browser downloaded it.",
  keptInstead: "{reason} The browser downloaded it instead.",
  notVideoPage: "Downpipe does not download videos from this page.",
  unknownVideoPage: "This is not a video page Downpipe knows.",
  mediaGone: "This media is no longer listed. Play it again.",
  mediaInvalid: "This media cannot be downloaded.",
  streamInvalid: "This stream cannot be downloaded.",
};
