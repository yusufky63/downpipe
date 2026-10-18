// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are. Switch names quote the popup.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "Filtros · Downpipe",
  headLabel: "Extensão do navegador",
  lede:
    "Escolha quais downloads o navegador entrega ao Downpipe enquanto Capturar downloads do navegador está ativado, e quais mídias a extensão lista enquanto Mídia nesta página está ativado. Todo o resto fica no navegador, como de costume.",
  captureOff:
    "A captura está desativada. Estes filtros valem quando você ativar Capturar downloads do navegador no pop-up do Downpipe na barra de ferramentas.",
  loadError: "Não foi possível carregar as configurações. Recarregue esta página para tentar novamente.",
  typesHeading: "01 — Tipos de arquivo",
  captureAll: "Capturar todos os tipos de arquivo",
  captureAllDetail: "Todo download que o navegador iniciar vai para o Downpipe, seja qual for o tipo.",
  typesLabel: "Somente estes tipos de arquivo",
  typesHelp:
    "Extensões separadas por espaços ou vírgulas, como {example}. Um download é capturado quando o nome do arquivo termina com uma delas; {tarGz} também funciona.",
  restoreDefault: "Restaurar a lista padrão",
  sitesHeading: "02 — Sites excluídos",
  sitesLabel: "Nunca capturar nem listar mídia nestes sites",
  sitesHelp:
    "Um site por linha. {wildcard} também cobre todos os subdomínios. Um download fica no navegador quando vem de um destes sites ou começa em uma das páginas deles, e a mídia que eles reproduzem não é listada. Portas e caminhos são ignorados.",
  sizeHeading: "03 — Tamanho mínimo",
  sizeLabel: "Deixar arquivos menores para o navegador",
  sizeHelp: "0 desativa esta opção. Quando o servidor não informa o tamanho de um arquivo, ele vai para o Downpipe.",
  mediaHeading: "04 — Mídia nas páginas",
  mediaHelp:
    "Usado enquanto Mídia nesta página está ativado no pop-up da barra de ferramentas. Transmissões HLS e DASH são sempre listadas; os sites excluídos acima também valem aqui.",
  overlay: "Mostrar um botão Baixar nos vídeos",
  overlayDetail: "Em um vídeo em reprodução, por alguns segundos e enquanto o ponteiro estiver sobre ele.",
  mediaTypesLabel: "Listar arquivos de mídia destes tipos",
  mediaTypesHelp:
    "Extensões separadas por espaços ou vírgulas. Um arquivo sem extensão é avaliado pelo tipo que o servidor envia. Uma lista vazia lista somente transmissões.",
  mediaSizeLabel: "Deixar de fora arquivos de mídia menores que",
  mediaSizeHelp:
    "Mantém de fora sons de notificação e prévias curtas. 0 lista todos os tamanhos; arquivos cujo tamanho o servidor não informa são sempre listados.",
  save: "Salvar",
  moreErrors: { one: "E mais {count}.", many: "E mais {count}.", other: "E mais {count}." },
  unsaved: "Alterações não salvas",
  notSavedFix: "Não salvo. Corrija os campos marcados em vermelho.",
  saving: "Salvando…",
  notSavedNoAnswer: "Não salvo. A extensão não respondeu; tente novamente.",
  saved: "Salvo",
  // {value} is what the user typed.
  errors: {
    notExtension: "\"{value}\" não é uma extensão de arquivo.",
    tooManyTypes: {
      one: "Liste no máximo {count} tipo de arquivo.",
      many: "Liste no máximo {count} tipos de arquivo.",
      other: "Liste no máximo {count} tipos de arquivo.",
    },
    notSite: "Linha {line}: \"{value}\" não é um site. Use um host como example.com ou *.example.com.",
    tooManySites: {
      one: "Liste no máximo {count} site.",
      many: "Liste no máximo {count} sites.",
      other: "Liste no máximo {count} sites.",
    },
    sizeNotNumber: "Digite um número como 2 ou 1.5, ou 0 para desativar esta opção.",
    sizeTooBig: "Digite um tamanho menor que 1 TiB.",
    noTypes: "Adicione pelo menos um tipo de arquivo ou ative Capturar todos os tipos de arquivo.",
  },
};
