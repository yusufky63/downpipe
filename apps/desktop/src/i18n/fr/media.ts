// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Sous-titres",
    none: "Aucun",
    uploaded: "De l'auteur",
    auto: "Sous-titres automatiques",
    original: "{language} (original)",
    embedded: "Les sous-titres sont intégrés au fichier vidéo.",
    separate: "Les sous-titres sont enregistrés à côté de la vidéo, dans un fichier.",
  },
  siteErrors: {
    forbidden: "YouTube a refusé le téléchargement. Une version plus récente de yt-dlp règle souvent le problème : mettez-la à jour dans Paramètres, Médias, puis réessayez.",
    signIn: "YouTube demande un navigateur connecté pour ce téléchargement. Downpipe n'utilise pas les cookies du navigateur : patientez un moment ou utilisez un autre réseau, puis réessayez.",
    ageRestricted: "YouTube ne montre cette vidéo qu'aux adultes connectés. Downpipe n'utilise ni compte ni cookies du navigateur ; il ne peut donc pas la télécharger.",
    unavailable: "Cette vidéo n'est pas disponible : elle est privée, supprimée, réservée aux membres ou bloquée dans votre région.",
    jsRuntime: "yt-dlp n'a pas pu répondre aux défis JavaScript de YouTube. Téléchargez Deno dans Paramètres, Médias, puis réessayez.",
    poToken: "YouTube a demandé un jeton de preuve d'origine (PO) pour cette vidéo. Mettez à jour yt-dlp dans Paramètres, Médias, ou choisissez une autre qualité.",
    outdated: "yt-dlp ne comprend plus cette page ; YouTube l'a probablement modifiée. Mettez à jour yt-dlp dans Paramètres, Médias, puis réessayez.",
  },
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
