// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "Annuler",
  save: "Enregistrer",
  close: "Fermer",
  browse: "Parcourir",
  dismiss: "Ignorer",
  keep: "Conserver",
  open: "Ouvrir",
  pause: "Suspendre",
  resume: "Reprendre",
  retry: "Réessayer",
  remove: "Supprimer",
  showInFolder: "Afficher dans le dossier",
  moreActions: "Plus d'actions",
  unknown: "Inconnu",
  none: "–",
  siteVideo: "Vidéo {site}",
  downloads: { one: "{count} téléchargement", many: "{count} téléchargements", other: "{count} téléchargements" },
  links: { one: "{count} lien", many: "{count} liens", other: "{count} liens" },
};

export const states: Messages["states"] = {
  confirm: "À confirmer",
  queued: "En attente",
  starting: "Connexion",
  downloading: "Téléchargement",
  waiting: "Nouvelle tentative",
  verifying: "Vérification",
  paused: "Suspendu",
  completed: "Terminé",
  failed: "Échec",
  canceled: "Annulé",
  filters: {
    all: "Tous",
    active: "Actifs",
    queued: "En attente",
    completed: "Terminés",
    failed: "En échec",
  },
};
