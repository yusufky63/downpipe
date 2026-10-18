// Videos and audio detected on pages, in the popup's media list and the
// overlay's menu. {site} is a site's own name (YouTube), {format} a file
// format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  hls: "Flux HLS",
  dash: "Flux DASH",
  hlsQualities: { one: "HLS · {count} qualité", many: "HLS · {count} qualités", other: "HLS · {count} qualités" },
  fileAudio: "Audio {format}",
  fileVideo: "Vidéo {format}",
  audio: "Audio",
  video: "Vidéo",
  siteVideo: "Vidéo {site}",
  thisSiteVideo: "Cette vidéo {site}",
  siteVideoQuality: "Vidéo {site} · qualité choisie dans Downpipe",
  updateForStreams: "Mettez à jour Downpipe pour télécharger des flux",
  updateForPages: "Mettez à jour Downpipe pour télécharger les vidéos des pages web",
  emptyOff:
    "Activez cette option pour lister les vidéos et les sons lus par les pages. Le navigateur demande d'abord l'accès à tous les sites.",
  emptyExcluded: "Non listé ici : {site} est un site exclu dans Filtres.",
  emptyNothing: "Rien pour l'instant. Lancez la lecture de la vidéo ; si elle était déjà en cours, rechargez la page.",
};
