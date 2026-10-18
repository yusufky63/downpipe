// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the "run:" texts are not translated.
export const status = {
  checking: "Checking the connection",
  connected: "Connected to Downpipe",
  connectedDetail: "App {app}, browser bridge {bridge}",
  appNotRunning: "Downpipe is not running",
  appNotRunningDetail: "Open the Downpipe app. Links sent while it is closed are not queued.",
  hostMissing: "Browser bridge not installed",
  hostMissingDetail: "Open the Downpipe app once; it registers the bridge when it starts. For a build from source, run:",
  hostForbidden: "Bridge does not allow this extension",
  hostForbiddenDetail: "Register it for this extension's id, then restart the browser:",
  error: "Bridge error",
};
