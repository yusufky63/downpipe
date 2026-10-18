// System notifications after a download was handed over, or not, and the
// toolbar button's tooltip. The download's name comes first in each message.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "In attesa della tua risposta in Downpipe",
  waitingBody: "Conferma il nome e la cartella in Downpipe per avviare il download.",
  downloading: "Download in corso in Downpipe",
  duplicate: "Già in Downpipe",
  kept: "Scaricato dal browser",
  failed: "Non inviato a Downpipe",
  toolbarTitle: "Downpipe: {label}. {message}",
};
