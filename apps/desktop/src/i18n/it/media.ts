// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Solo audio",
  defaultQuality: "Predefinita",
  aboutSize: "circa {size}",
  audio: "Audio",
  tools: {
    found: "Trovato",
    notFound: "Non trovato",
    problem: "Problema",
    version: "versione {version}",
    sources: {
      settings: "impostato nelle Impostazioni",
      managed: "scaricato da Downpipe",
      path: "nel PATH",
    },
    missing: {
      ytDlp: "Necessario per i video nelle pagine.",
      runtime: "Senza Deno (o Node), yt-dlp non trova alcune qualità di YouTube.",
      ffmpeg: "Senza FFmpeg funzionano solo l'opzione Solo audio e le qualità con un proprio audio.",
    },
    download: "Scarica {tools}",
    update: "Aggiorna {tools}",
  },
};
