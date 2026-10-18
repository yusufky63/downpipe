// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Solo audio",
  defaultQuality: "Predeterminada",
  aboutSize: "aprox. {size}",
  audio: "Audio",
  tools: {
    found: "Encontrado",
    notFound: "No encontrado",
    problem: "Problema",
    version: "versión {version}",
    sources: {
      settings: "definido en Configuración",
      managed: "descargado por Downpipe",
      path: "en PATH",
    },
    missing: {
      ytDlp: "Necesario para videos en páginas.",
      runtime: "Sin Deno (o Node), yt-dlp no obtiene algunas calidades de YouTube.",
      ffmpeg: "Sin él, únicamente funcionan Solo audio y las calidades con sonido propio.",
    },
    download: "Descargar {tools}",
    update: "Actualizar {tools}",
  },
};
