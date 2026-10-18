// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the "run:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Verbindung wird geprüft",
  connected: "Mit Downpipe verbunden",
  connectedDetail: "App {app}, Browser-Anbindung {bridge}",
  appNotRunning: "Downpipe wird nicht ausgeführt",
  appNotRunningDetail: "Öffnen Sie die Downpipe-App. Links, die gesendet werden, während sie geschlossen ist, werden nicht vorgemerkt.",
  hostMissing: "Browser-Anbindung nicht installiert",
  hostMissingDetail:
    "Öffnen Sie die Downpipe-App einmal; sie registriert die Anbindung beim Start. Führen Sie bei einem Build aus dem Quellcode Folgendes aus:",
  hostForbidden: "Anbindung lässt diese Erweiterung nicht zu",
  hostForbiddenDetail: "Registrieren Sie sie für die ID dieser Erweiterung, und starten Sie dann den Browser neu:",
  error: "Fehler der Anbindung",
};
