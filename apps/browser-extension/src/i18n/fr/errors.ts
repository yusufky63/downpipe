// Problems the extension words itself. Messages the app sends over the
// browser bridge for codes not listed here are shown as the app wrote them.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "La passerelle de navigateur de Downpipe n'est pas installée.",
    host_forbidden: "La passerelle de navigateur est installée mais n'autorise pas cette extension.",
    host_error: "La passerelle de navigateur s'est arrêtée de manière inattendue.",
    timeout: "Downpipe n'a pas répondu à temps.",
    bad_response: "La passerelle de navigateur a envoyé une réponse que cette extension ne comprend pas.",
  },
  code: {
    app_not_running: "Downpipe n'est pas en cours d'exécution. Ouvrez l'application Downpipe et réessayez.",
    app_unavailable: "Downpipe n'a pas répondu. Redémarrez l'application et réessayez.",
    duplicate: "Déjà dans Downpipe.",
    invalid_url: "Ce lien ne peut pas être téléchargé. Seuls les liens http et https fonctionnent.",
    version_mismatch:
      "Downpipe et sa passerelle de navigateur n'ont pas la même version. Redémarrez Downpipe pour qu'il enregistre sa propre passerelle.",
    unsupported_version: "Cette extension est plus récente que la version de Downpipe installée. Mettez à jour Downpipe.",
    message_too_large: "Le lien est trop long pour être transmis.",
    unsupported_page: "Downpipe ne télécharge pas les vidéos de ce site.",
  },
  refused: "Downpipe a refusé le téléchargement ({code}).",
  streamsUnsupported: "Cette version de Downpipe ne peut pas télécharger de flux. Mettez à jour Downpipe.",
  pagesUnsupported: "Cette version de Downpipe ne peut pas télécharger les vidéos des pages web. Mettez à jour Downpipe.",
  keptClosed: "Downpipe n'était pas en cours d'exécution : le navigateur a donc téléchargé le fichier.",
  keptInstead: "{reason} Le navigateur l'a téléchargé à la place.",
  notVideoPage: "Downpipe ne télécharge pas les vidéos de cette page.",
  unknownVideoPage: "Cette page n'est pas une page vidéo connue de Downpipe.",
  mediaGone: "Ce média ne figure plus dans la liste. Relancez sa lecture.",
  mediaInvalid: "Ce média ne peut pas être téléchargé.",
  streamInvalid: "Ce flux ne peut pas être téléchargé.",
};
