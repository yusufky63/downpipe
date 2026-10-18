// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Audio uniquement",
  defaultQuality: "Par défaut",
  aboutSize: "environ {size}",
  audio: "Audio",
  tools: {
    found: "Trouvé",
    notFound: "Introuvable",
    problem: "Problème",
    version: "version {version}",
    sources: {
      settings: "défini dans les Paramètres",
      managed: "téléchargé par Downpipe",
      path: "dans le PATH",
    },
    missing: {
      ytDlp: "Nécessaire pour les vidéos sur les pages web.",
      runtime: "Sans Deno (ou Node), yt-dlp ne trouve pas certaines qualités YouTube.",
      ffmpeg: "Sans lui, seules la qualité Audio uniquement et les qualités qui ont leur propre son fonctionnent.",
    },
    // {tools} comes from Intl.ListFormat, e.g. "yt-dlp et Deno".
    download: "Télécharger {tools}",
    update: "Mettre à jour {tools}",
  },
};
