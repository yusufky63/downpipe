// Problems the extension words itself. Messages the app sends for other codes are shown as the app wrote them.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "A ponte com o navegador do Downpipe não está instalada.",
    host_forbidden: "A ponte com o navegador está instalada, mas não permite esta extensão.",
    host_error: "A ponte com o navegador parou inesperadamente.",
    timeout: "O Downpipe não respondeu a tempo.",
    bad_response: "A ponte com o navegador enviou uma resposta que esta extensão não entende.",
  },
  code: {
    app_not_running: "O Downpipe não está em execução. Abra o aplicativo Downpipe e tente novamente.",
    app_unavailable: "O Downpipe não respondeu. Reinicie o aplicativo e tente novamente.",
    duplicate: "Já está no Downpipe.",
    invalid_url: "Este link não pode ser baixado. Somente links http e https funcionam.",
    version_mismatch:
      "O Downpipe e a ponte com o navegador são de versões diferentes. Reinicie o Downpipe para que ele registre a própria ponte.",
    unsupported_version: "Esta extensão é mais nova que o Downpipe instalado. Atualize o Downpipe.",
    message_too_large: "O link é longo demais para ser enviado.",
    unsupported_page: "O Downpipe não baixa vídeos deste site.",
  },
  refused: "O Downpipe recusou o download ({code}).",
  streamsUnsupported: "Esta versão do Downpipe não baixa transmissões. Atualize o Downpipe.",
  pagesUnsupported: "Esta versão do Downpipe não baixa vídeos de páginas. Atualize o Downpipe.",
  keptClosed: "O Downpipe não estava em execução, então o navegador fez o download.",
  keptInstead: "{reason} O navegador fez o download no lugar dele.",
  notVideoPage: "O Downpipe não baixa vídeos desta página.",
  unknownVideoPage: "Esta não é uma página de vídeo que o Downpipe conhece.",
  mediaGone: "Esta mídia não está mais na lista. Reproduza-a novamente.",
  mediaInvalid: "Esta mídia não pode ser baixada.",
  streamInvalid: "Esta transmissão não pode ser baixada.",
};
