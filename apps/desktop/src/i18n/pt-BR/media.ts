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
      ffmpegManual: "O Downpipe não consegue baixar o FFmpeg para este sistema. Instale-o (no macOS: brew install ffmpeg) e informe o caminho no topo desta aba, por exemplo /opt/homebrew/bin/ffmpeg.",
    },
    // {tools} is a list such as "yt-dlp e Deno", joined by Intl.ListFormat.
    download: "Baixar",
    update: "Atualizar",
    installAll: "Instalar tudo (cerca de {size})",
    offer: "Baixar as ferramentas que faltam (cerca de {size})",
    missingForVideo: "Falta para este vídeo: {tools}.",
    missingForQuality: "Falta para esta qualidade: {tools}.",
    starting: "Iniciando o download…",
    progress: "Baixando {tool}… {done} de {total}",
    progressUnknown: "Baixando {tool}… {done}",
    installing: "Instalando {tool}…",
    sourceNote:
      "Os lançamentos oficiais no GitHub, cada um verificado com o SHA-256 publicado no respectivo lançamento. Nada é baixado antes de você clicar.",
    checked: "Última busca por uma versão nova: {time}",
    neverChecked: "Ainda não houve busca por uma versão nova. O Downpipe verifica uma vez por dia, quando o download de um vídeo começa.",
    autoUpdate: "Manter o yt-dlp atualizado",
    autoUpdateHint:
      "Uma vez por dia, quando o download de um vídeo começa, o Downpipe procura um yt-dlp mais recente e o instala, verificado com o SHA-256 dele como um download feito aqui. Só a cópia baixada pelo Downpipe é substituída, nunca uma sua.",
  },
};
