// The details dialog: progress, live numbers, the new-link form and the file's facts.
import type { Messages } from "../core";

export const details: Messages["details"] = {
  eyebrow: "Detalhes do download",
  openFile: "Abrir arquivo",
  remove: "Remover…",
  cancelAndDelete: "Cancelar e excluir",
  cancelAndDeleteTitle: "Para o download e exclui o arquivo parcial",
  downloadAgain: "Baixar novamente",
  redownload: "Baixar novamente…",
  progress: "Progresso",
  progressOf: "{downloaded} de {total}",
  progressOfAbout: "{downloaded} de cerca de {total}",
  legend: {
    downloaded: "Baixado",
    connection: "Conexão",
    remaining: "Restante",
  },
  stats: {
    speed: "Velocidade",
    timeLeft: "Tempo restante",
    connections: "Conexões",
    target: "meta {count}",
    resume: "Retomada",
    // Gender-neutral, so it reads right next to "Retomada" and beside common.unknown.
    supported: "Com suporte",
    notSupported: "Sem suporte",
    size: "Tamanho",
    downloaded: "Baixado",
  },
  rows: {
    savedTo: "Salvo em",
    folder: "Pasta",
    source: "Origem",
    video: "Vídeo",
    stream: "Transmissão",
    bestQuality: "Melhor qualidade",
    fragments: { one: "{done} de {count} fragmento", many: "{done} de {count} fragmentos", other: "{done} de {count} fragmentos" },
    queue: "Fila",
    siteRule: "Regra do site",
    checksum: "Soma de verificação",
    added: "Adicionado",
    finished: "Concluído",
    speedLimit: "Limite de velocidade",
    resumes: "Retoma",
  },
  relink: {
    button: "Usar um novo link…",
    label: "Novo link para este arquivo",
    submit: "Continuar",
    hint: "Para um link expirado ou quebrado. O download continua de onde parou somente se o novo link fornecer o mesmo arquivo; caso contrário, nada é sobrescrito.",
  },
  copy: "Copiar",
  copied: "Copiado",
  /** The download's own speed limit, set in place. */
  speedLimit: {
    none: "Só o limite global",
    change: "Alterar",
    set: "Definir",
    remove: "Remover",
    label: "Limite de velocidade deste download",
  },
};
