// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings. Codec, tool and site names,
// resolutions and units stay as they are.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "Legendas",
    none: "Nenhuma",
    uploaded: "Do autor",
    auto: "Legendas automáticas",
    original: "{language} (original)",
    embedded: "As legendas vão dentro do arquivo de vídeo.",
    separate: "As legendas são salvas ao lado do vídeo, em um arquivo.",
  },
  siteErrors: {
    forbidden: "O YouTube recusou o download. Uma versão mais nova do yt-dlp costuma resolver: atualize-o em Configurações, Mídia e tente de novo.",
    signIn: "O YouTube quer um navegador conectado para este download. O Downpipe não usa cookies do navegador: espere um pouco ou use outra rede e tente de novo.",
    ageRestricted: "O YouTube mostra este vídeo apenas para adultos conectados. O Downpipe não usa contas nem cookies do navegador, então não consegue baixá-lo.",
    unavailable: "Este vídeo não está disponível: ele é privado, foi removido, é exclusivo para membros ou está bloqueado na sua região.",
    jsRuntime: "O yt-dlp não conseguiu responder aos desafios de JavaScript do YouTube. Baixe o Deno em Configurações, Mídia e tente de novo.",
    poToken: "O YouTube pediu um token de prova de origem (PO) para este vídeo. Atualize o yt-dlp em Configurações, Mídia ou escolha outra qualidade.",
    outdated: "O yt-dlp não entende mais esta página; provavelmente o YouTube a mudou. Atualize o yt-dlp em Configurações, Mídia e tente de novo.",
  },
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
