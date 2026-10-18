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
    invalid_message: "Downpipe could not read what the extension sent. Update Downpipe and the extension.",
    cookies_not_accepted: "Downpipe never accepts cookies from the browser.",
    internal: "Something went wrong inside Downpipe.",
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
  /** The engine's failure kinds, for downloads the app could not finish. */
  serverError: "The server answered with an error.",
  kind: {
    invalid_header: "One of this download's custom headers is not valid.",
    auth_required: "The server asks for a user name and password.",
    forbidden: "The server refused the download. The link may have expired, or the site may need you to sign in.",
    not_found: "The file was not found on the server. Check the link or use a new one.",
    throttled: "The server is limiting downloads. Try again later or with fewer connections.",
    http_status: "The server answered with an error (HTTP {status}).",
    resource_changed: "The file on the server changed during the download. Download it again from the start.",
    different_file: "The new link leads to a different file. The downloaded part was kept.",
    expired_link: "The link has expired: the server sent a web page instead of the file. Use a new link; the downloaded part is kept.",
    login_required: "The server asks you to sign in. Sign in in your browser, then use a new link; the downloaded part is kept.",
    range_not_honoured: "The server stopped sending the file in parts. Try again with one connection.",
    network: "The connection to the server failed. Check your network and try again.",
    checksum: "The downloaded file does not match its checksum. It may be damaged; download it again.",
    io: "The file could not be written. Check that the folder exists and the disk has free space.",
    corrupt_meta: "The resume data of this download is damaged. Download it again from the start.",
    drm_protected: "This stream is protected by DRM. Downpipe does not download protected media.",
    live_stream: "This is a live stream. Only finished streams can be downloaded.",
    invalid_manifest: "The stream's playlist could not be read.",
    unsupported_media: "This kind of stream is not supported.",
    tool_missing: "A program this download needs (yt-dlp, Deno or FFmpeg) is not set up. Set it up in Downpipe under Preferences, Media.",
    unavailable: "The site does not offer this video: it may be private, removed, or restricted by age or region.",
    extractor: "yt-dlp could not read this video page.",
  },
};
