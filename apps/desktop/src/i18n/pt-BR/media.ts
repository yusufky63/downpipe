// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "Somente áudio",
  defaultQuality: "Padrão",
  aboutSize: "cerca de {size}",
  audio: "Áudio",
  tools: {
    found: "Encontrado",
    notFound: "Não encontrado",
    problem: "Problema",
    version: "versão {version}",
    sources: {
      settings: "definido nas Configurações",
      managed: "baixado pelo Downpipe",
      path: "no PATH",
    },
    missing: {
      ytDlp: "Necessário para vídeos em páginas.",
      runtime: "Sem o Deno (ou o Node), o yt-dlp não obtém algumas qualidades do YouTube.",
      ffmpeg: "Sem ele, só funcionam a opção Somente áudio e as qualidades com som próprio.",
    },
    // {tools} is a list such as "yt-dlp e Deno", joined by Intl.ListFormat.
    download: "Baixar {tools}",
    update: "Atualizar {tools}",
  },
};
