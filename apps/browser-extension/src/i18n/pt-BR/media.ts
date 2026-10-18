// Videos and audio detected on pages. {site} is a site's own name (YouTube), {format} a file format (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  hls: "Transmissão HLS",
  dash: "Transmissão DASH",
  hlsQualities: { one: "HLS · {count} qualidade", many: "HLS · {count} qualidades", other: "HLS · {count} qualidades" },
  fileAudio: "Áudio {format}",
  fileVideo: "Vídeo {format}",
  audio: "Áudio",
  video: "Vídeo",
  siteVideo: "Vídeo do {site}",
  thisSiteVideo: "Este vídeo do {site}",
  siteVideoQuality: "Vídeo do {site} · qualidade escolhida no Downpipe",
  updateForStreams: "Atualize o Downpipe para baixar transmissões",
  updateForPages: "Atualize o Downpipe para baixar vídeos de páginas",
  emptyOff:
    "Ative para listar os vídeos e áudios que as páginas reproduzem. Antes, o navegador pede acesso a todos os sites.",
  emptyExcluded: "Não listado aqui: {site} é um site excluído em Filtros.",
  emptyNothing: "Nada ainda. Reproduza o vídeo; se ele já estava tocando, recarregue a página.",
};
