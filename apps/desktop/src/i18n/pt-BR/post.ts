// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "Verificando",
    extracting: "Extraindo",
    running_command: "Executando comando",
  },
  threat: "Ameaça",
  summary: {
    threat: "Ameaça encontrada: {threat}",
    notExtracted: "Não extraído",
    commandFailed: "O comando após o download falhou",
  },
  section: "Após o download",
  scan: {
    label: "Verificação de vírus",
    clean: "{scanner} não encontrou nada.",
    threat:
      "{scanner} encontrou {threat}. Nada foi extraído nem executado, e o arquivo foi deixado como estava: excluí-lo é decisão sua.",
    unnamed: "uma ameaça",
    theScanner: "O antivírus",
    missing: "Não verificado: nenhum antivírus configurado. Escolha um em Configurações, Após o download.",
    missingNamed: "Não verificado: {scanner} não foi encontrado.",
    failed: "{scanner} não conseguiu verificar o arquivo: {detail}",
  },
  extract: {
    label: "Extraído",
    done: { one: "{count} arquivo em uma nova pasta.", many: "{count} arquivos em uma nova pasta.", other: "{count} arquivos em uma nova pasta." },
    skipped: {
      one: "{count} item foi deixado de fora: um nome inseguro, um link ou uma duplicata.",
      many: "{count} itens foram deixados de fora: nomes inseguros, links ou duplicatas.",
      other: "{count} itens foram deixados de fora: nomes inseguros, links ou duplicatas.",
    },
    deleted: "O arquivo compactado foi excluído depois.",
    openFolder: "Abrir pasta",
    failed: "Não extraído: {reason}.",
    errors: {
      encrypted: "o arquivo compactado é protegido por senha",
      too_large: "ele se expandiria muito além do próprio tamanho (uma possível bomba zip)",
      too_many_entries: "ele tem itens demais",
      corrupt: "o arquivo compactado está danificado ou usa um método sem suporte",
      io: "não foi possível gravar os arquivos",
    },
  },
  command: {
    label: "Comando",
    ok: "{program} terminou.",
    failed: "{program} terminou com o código de saída {code}.",
    ended: "{program} foi encerrado antes de terminar.",
    timedOut: "{program} passou do tempo limite e foi interrompido.",
    notStarted: "Não foi possível iniciar {program}: {detail}",
    invalid: "O comando não pode ser executado: {detail}",
    theCommand: "O comando",
    output: "Saída",
  },
  fields: {
    heading: "Após o download",
    command: "Executar ao terminar",
    inherited: "Padrão: {command}",
    nothing: "Nada",
    hint: "Um programa e seus argumentos, como notepad {path}. {path}, {name}, {folder} e {url} são preenchidos, cada um como um único argumento, sem shell. Não é executado se o download falhar.",
    extract: "Extrair o arquivo compactado",
    extractHint: "Em uma nova pasta ao lado dele, quando terminar.",
    save: "Salvar",
    saved: "Salvo. Vale quando o download terminar.",
  },
  settings: {
    tab: "Após o download",
    command: "Executar um comando após cada download",
    commandHint:
      "Um programa e seus argumentos, como notepad {path}. {path}, {name}, {folder} e {url} são preenchidos, cada um como um único argumento; nada passa por um shell, então nomes de arquivo e links não podem acrescentar comandos. Filas e downloads individuais podem definir o seu. Não é executado se um download falhar.",
    timeout: "Tempo limite",
    timeoutHint: "Segundos que um comando ou uma verificação pode durar antes de ser interrompido, com tudo o que iniciou. De 5 a 86400.",
    seconds: "s",
    scan: "Verificar vírus nos downloads",
    scanHintWindows:
      "Com o Microsoft Defender, após cada download. Uma ameaça marca o download e mostra uma notificação; nada é excluído nem posto em quarentena, e nada é extraído nem executado.",
    scanHintOther:
      "Com o antivírus abaixo, após cada download. Uma ameaça marca o download e mostra uma notificação; nada é excluído, extraído nem executado.",
    scanner: "Antivírus",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Vazio usa o Microsoft Defender no Windows. Qualquer outro antivírus recebe o arquivo como {path} ou como último argumento; o código de saída 0 significa limpo e 1 uma ameaça, como no clamscan do ClamAV.",
    archives: "Arquivos compactados",
    archivesHint:
      "Arquivos zip, tar, tar.gz, tar.xz e 7z são extraídos em uma nova pasta ao lado deles quando a categoria pede (Categorias) ou o download pede. Itens que ficariam fora dessa pasta, links e arquivos que se expandem muito além do próprio tamanho são recusados. RAR não tem suporte; o 7-Zip pode extraí-lo como comando.",
    deleteArchive: "Excluir o arquivo compactado depois de extraí-lo",
    deleteArchiveHint: "Só quando todos os itens foram extraídos, e depois que o comando rodou.",
    queueCommand: "Comando após os downloads de {name}",
    queueCommandUnnamed: "Comando após os downloads da fila",
    queueCommandPlaceholder: "Comando ao terminar; vazio: o de Após o download",
    categoryExtract: "Extrair arquivos compactados",
    categoryExtractTitle: "Extrair os arquivos zip, tar e 7z desta categoria após o download",
  },
};
