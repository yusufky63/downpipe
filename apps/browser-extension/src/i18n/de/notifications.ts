// System notifications after a download was handed over, or not, and the
// toolbar button's tooltip. The download's name comes first in each message.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "Wartet in Downpipe auf Ihre Antwort",
  waitingBody: "Bestätigen Sie Name und Ordner in Downpipe, um den Download zu starten.",
  downloading: "Wird in Downpipe heruntergeladen",
  duplicate: "Bereits in Downpipe",
  kept: "Vom Browser heruntergeladen",
  failed: "Nicht an Downpipe gesendet",
  toolbarTitle: "Downpipe: {label}. {message}",
};
