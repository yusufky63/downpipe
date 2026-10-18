// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are in every language.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Sottotitoli",
    none: "Nessuno",
    uploaded: "Dell'autore",
    auto: "Sottotitoli automatici",
    original: "{language} (originale)",
    embedded: "I sottotitoli vengono inseriti nel file video.",
    separate: "I sottotitoli vengono salvati accanto al video, in un file.",
  },
  siteErrors: {
    forbidden: "YouTube ha rifiutato il download. Di solito una versione più recente di yt-dlp risolve: aggiornala in Impostazioni, Media, poi riprova.",
    signIn: "Per questo download YouTube chiede un browser con accesso effettuato. Downpipe non usa i cookie del browser: aspetta un po' o usa un'altra rete, poi riprova.",
    ageRestricted: "YouTube mostra questo video solo agli adulti che hanno effettuato l'accesso. Downpipe non usa account né cookie del browser, quindi non può scaricarlo.",
    unavailable: "Questo video non è disponibile: è privato, rimosso, riservato agli abbonati o bloccato nella tua zona.",
    jsRuntime: "yt-dlp non è riuscito a rispondere alle verifiche JavaScript di YouTube. Scarica Deno in Impostazioni, Media, poi riprova.",
    poToken: "YouTube ha chiesto un token di prova di origine (PO) per questo video. Aggiorna yt-dlp in Impostazioni, Media, o scegli un'altra qualità.",
    outdated: "yt-dlp non capisce più questa pagina; probabilmente YouTube l'ha cambiata. Aggiorna yt-dlp in Impostazioni, Media, poi riprova.",
  },
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
