// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the texts ending in ":" are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Vérification de la connexion",
  connected: "Connecté à Downpipe",
  connectedDetail: "Application {app}, passerelle de navigateur {bridge}",
  appNotRunning: "Downpipe n'est pas en cours d'exécution",
  /** Instead of appNotRunningDetail while the bridge may start Downpipe. */
  appNotRunningStarts: "Un téléchargement envoyé le lance dans la zone de notification.",
  appNotRunningDetail:
    "Ouvrez l'application Downpipe. Les liens envoyés pendant qu'elle est fermée ne sont pas mis en file d'attente.",
  hostMissing: "Passerelle de navigateur non installée",
  hostMissingDetail:
    "Ouvrez une fois l'application Downpipe ; elle enregistre la passerelle à son démarrage. Pour une version compilée à partir des sources, exécutez :",
  hostForbidden: "La passerelle n'autorise pas cette extension",
  hostForbiddenDetail: "Enregistrez-la pour l'identifiant de cette extension, puis redémarrez le navigateur :",
  error: "Erreur de la passerelle",
};
