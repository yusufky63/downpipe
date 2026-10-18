// System notifications after a download was handed over, or not, and the
// toolbar button's tooltip. The download's name comes first in each message.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "En attente de votre réponse dans Downpipe",
  waitingBody: "Confirmez son nom et son dossier dans Downpipe pour démarrer le téléchargement.",
  downloading: "Téléchargement dans Downpipe",
  duplicate: "Déjà dans Downpipe",
  kept: "Téléchargé par le navigateur",
  failed: "Non envoyé à Downpipe",
  toolbarTitle: "Downpipe : {label}. {message}",
};
