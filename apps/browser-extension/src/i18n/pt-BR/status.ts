// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the "execute:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Verificando a conexão",
  connected: "Conectado ao Downpipe",
  connectedDetail: "Aplicativo {app}, ponte com o navegador {bridge}",
  appNotRunning: "O Downpipe não está em execução",
  /** Instead of appNotRunningDetail while the bridge may start Downpipe. */
  appNotRunningStarts: "Um download enviado o inicia na bandeja do sistema.",
  appNotRunningDetail: "Abra o aplicativo Downpipe. Links enviados enquanto ele está fechado não entram na fila.",
  hostMissing: "Ponte com o navegador não instalada",
  hostMissingDetail:
    "Abra o aplicativo Downpipe uma vez; ele registra a ponte ao iniciar. Para uma compilação a partir do código-fonte, execute:",
  hostForbidden: "A ponte não permite esta extensão",
  hostForbiddenDetail: "Registre-a para o id desta extensão e reinicie o navegador:",
  error: "Erro na ponte",
};
