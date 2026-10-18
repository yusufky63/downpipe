// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "Playlist {site}",
    channel: "Chaîne {site}",
  },
  untitled: "Playlist",
  reading: "Demande des vidéos à yt-dlp…",
  unavailable: "Cette playlist ou cette chaîne n'est pas disponible : elle est privée, supprimée ou bloquée dans votre région.",
  empty: "yt-dlp n'a trouvé aucune vidéo dans cette liste.",
  truncated: { one: "Seule la première vidéo est listée.", many: "Seules les {count} premières vidéos sont listées.", other: "Seules les {count} premières vidéos sont listées." },
  filter: "Filtrer par titre",
  selectAll: "Tout sélectionner",
  selectNone: "Ne rien sélectionner",
  selected: { one: "{count} sur {total} sélectionnée", many: "{count} sur {total} sélectionnées", other: "{count} sur {total} sélectionnées" },
  rowsLabel: "Vidéos",
  notAvailable: "Non disponible",
  untitledVideo: "Vidéo sans titre",
  quality: "Qualité",
  qualityHint: "Une seule qualité pour toutes les vidéos ; une vidéo qui ne l'a pas prend la suivante en dessous.",
  presets: {
    best: "Meilleure",
    "1080p": "Jusqu'à 1080p",
    "720p": "Jusqu'à 720p",
    audio: "Audio uniquement",
  },
  autoCaptions: "Utiliser les sous-titres automatiques en l'absence de sous-titres",
  noFfmpeg: "FFmpeg introuvable : seules les qualités avec leur propre son peuvent être téléchargées, soit 360p au plus sur YouTube.",
  folder: "Dossier",
  folderHint: "Créé dans le dossier d'à côté. Vide : pas de dossier à part.",
  saveIn: "Enregistrer dans",
  submit: { one: "Télécharger {count} vidéo", many: "Télécharger {count} vidéos", other: "Télécharger {count} vidéos" },
  addToList: { one: "Ajouter {count} vidéo", many: "Ajouter {count} vidéos", other: "Ajouter {count} vidéos" },
  added: { one: "{count} vidéo ajoutée.", many: "{count} vidéos ajoutées.", other: "{count} vidéos ajoutées." },
  skipped: { one: "{count} était déjà dans la liste.", many: "{count} étaient déjà dans la liste.", other: "{count} étaient déjà dans la liste." },
};
