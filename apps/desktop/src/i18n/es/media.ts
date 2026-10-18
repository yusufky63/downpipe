// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Subtítulos",
    none: "Ninguno",
    uploaded: "Del autor",
    auto: "Subtítulos automáticos",
    original: "{language} (original)",
    embedded: "Los subtítulos se incluyen en el archivo de video.",
    separate: "Los subtítulos se guardan junto al video como un archivo.",
  },
  siteErrors: {
    forbidden: "YouTube rechazó la descarga. Una versión más reciente de yt-dlp suele solucionarlo: actualízala en Configuración, Multimedia y vuelve a intentarlo.",
    signIn: "YouTube pide un navegador con la sesión iniciada para esta descarga. Downpipe no usa las cookies del navegador: espera un rato o usa otra red y vuelve a intentarlo.",
    ageRestricted: "YouTube muestra este video solo a adultos con la sesión iniciada. Downpipe no usa cuentas ni cookies del navegador, así que no puede descargarlo.",
    unavailable: "Este video no está disponible: es privado, se eliminó, es exclusivo para miembros o está bloqueado en tu región.",
    jsRuntime: "yt-dlp no pudo responder a los desafíos de JavaScript de YouTube. Descarga Deno en Configuración, Multimedia y vuelve a intentarlo.",
    poToken: "YouTube pidió un token de prueba de origen (PO) para este video. Actualiza yt-dlp en Configuración, Multimedia o elige otra calidad.",
    outdated: "yt-dlp ya no entiende esta página; probablemente YouTube la cambió. Actualiza yt-dlp en Configuración, Multimedia y vuelve a intentarlo.",
  },
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
