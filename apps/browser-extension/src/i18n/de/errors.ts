// Problems the extension words itself. Messages the app sends over the
// browser bridge for codes not listed here are shown as the app wrote them.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "Die Browser-Anbindung von Downpipe ist nicht installiert.",
    host_forbidden: "Die Browser-Anbindung ist installiert, lässt diese Erweiterung aber nicht zu.",
    host_error: "Die Browser-Anbindung wurde unerwartet beendet.",
    timeout: "Downpipe hat nicht rechtzeitig geantwortet.",
    bad_response: "Die Browser-Anbindung hat eine Antwort gesendet, die diese Erweiterung nicht versteht.",
  },
  code: {
    app_not_running: "Downpipe wird nicht ausgeführt. Öffnen Sie die Downpipe-App, und versuchen Sie es erneut.",
    app_unavailable: "Downpipe hat nicht geantwortet. Starten Sie die App neu, und versuchen Sie es erneut.",
    duplicate: "Bereits in Downpipe.",
    invalid_url: "Dieser Link kann nicht heruntergeladen werden. Nur http- und https-Links funktionieren.",
    version_mismatch:
      "Downpipe und seine Browser-Anbindung haben unterschiedliche Versionen. Starten Sie Downpipe neu, damit es seine eigene Anbindung registriert.",
    unsupported_version: "Diese Erweiterung ist neuer als das installierte Downpipe. Aktualisieren Sie Downpipe.",
    message_too_large: "Der Link ist zu lang für die Übergabe.",
    unsupported_page: "Downpipe lädt keine Videos von dieser Website herunter.",
  },
  refused: "Downpipe hat den Download abgelehnt ({code}).",
  streamsUnsupported: "Diese Version von Downpipe kann keine Streams herunterladen. Aktualisieren Sie Downpipe.",
  pagesUnsupported: "Diese Version von Downpipe kann keine Videos von Webseiten herunterladen. Aktualisieren Sie Downpipe.",
  keptClosed: "Downpipe wurde nicht ausgeführt, daher hat der Browser die Datei heruntergeladen.",
  keptInstead: "{reason} Stattdessen hat der Browser die Datei heruntergeladen.",
  notVideoPage: "Downpipe lädt keine Videos von dieser Seite herunter.",
  unknownVideoPage: "Dies ist keine Videoseite, die Downpipe kennt.",
  mediaGone: "Dieses Medium ist nicht mehr aufgeführt. Spielen Sie es erneut ab.",
  mediaInvalid: "Dieses Medium kann nicht heruntergeladen werden.",
  streamInvalid: "Dieser Stream kann nicht heruntergeladen werden.",
};
