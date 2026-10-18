// Encontrar arquivos na web: o coletor de sites, as assinaturas de feeds e os hospedeiros de arquivos.
import type { Messages } from "../core";

export const grabber: Messages["grabber"] = {
  open: "Coletor de sites",
  eyebrow: "Coletor de sites",
  title: "Encontrar arquivos em um site",
  address: {
    label: "Endereço inicial",
    hint: "O Downpipe lê esta página e segue os links dela. Nenhum cookie é enviado, então ele vê o que um visitante sem login vê.",
  },
  depth: {
    label: "Profundidade",
    hint: "1 lê só esta página; cada nível segue mais um nível de links.",
  },
  pages: {
    label: "Limite de páginas",
    hint: "Máximo de páginas lidas",
  },
  types: {
    label: "Tipos de arquivo",
    custom: "Outros tipos",
    customPlaceholder: "ex.: iso, torrent",
  },
  sameSite: {
    label: "Ficar neste site",
    hint: "Só as páginas do site do endereço inicial são lidas. Arquivos com links de outros sites continuam sendo listados.",
  },
  robots: {
    label: "Respeitar o robots.txt",
    hint: "Pula as páginas que o site pede para os robôs evitarem.",
  },
  sizes: {
    label: "Consultar tamanhos",
    hint: "Uma pequena requisição por arquivo, para os primeiros {count} arquivos.",
  },
  delay: {
    label: "Intervalo",
    hint: "Milissegundos entre duas requisições ao mesmo site. Sites podem bloquear visitantes apressados.",
  },
  start: "Encontrar arquivos",
  stop: "Parar",
  again: "Nova busca",
  progress: {
    pages: { one: "{done} de {count} página lidas", many: "{done} de {count} páginas lidas", other: "{done} de {count} páginas lidas" },
    sizes: {
      one: "Consultando tamanhos: {done} de {count} arquivo",
      many: "Consultando tamanhos: {done} de {count} arquivos",
      other: "Consultando tamanhos: {done} de {count} arquivos",
    },
    files: { one: "{count} arquivo encontrado", many: "{count} arquivos encontrados", other: "{count} arquivos encontrados" },
  },
  results: {
    title: { one: "{count} arquivo encontrado", many: "{count} arquivos encontrados", other: "{count} arquivos encontrados" },
    none: "Nenhum arquivo desses tipos foi encontrado.",
    canceled: "Parado antes do fim: estes são os arquivos encontrados até então.",
    pageLimit: "O limite de páginas foi atingido antes de todas serem lidas.",
    blocked: {
      one: "{count} página ou arquivo foi pulado porque o robots.txt não permite.",
      many: "{count} páginas ou arquivos foram pulados porque o robots.txt não permite.",
      other: "{count} páginas ou arquivos foram pulados porque o robots.txt não permite.",
    },
    failed: { one: "Não foi possível ler {count} página.", many: "Não foi possível ler {count} páginas.", other: "Não foi possível ler {count} páginas." },
    filter: "Filtrar arquivos",
    selectAll: "Selecionar todos os arquivos mostrados",
    name: "Nome",
    size: "Tamanho",
    foundOn: "Encontrado em",
    selected: { one: "{count} selecionado, {size}", many: "{count} selecionados, {size}", other: "{count} selecionados, {size}" },
    unknownSizes: { one: "{count} de tamanho desconhecido", many: "{count} de tamanho desconhecido", other: "{count} de tamanho desconhecido" },
    add: { one: "Adicionar {count} arquivo", many: "Adicionar {count} arquivos", other: "Adicionar {count} arquivos" },
    added: { one: "{count} arquivo adicionado.", many: "{count} arquivos adicionados.", other: "{count} arquivos adicionados." },
    alreadyListed: { one: "{count} já estava na lista.", many: "{count} já estavam na lista.", other: "{count} já estavam na lista." },
  },
  errors: {
    invalid_url: "Digite um endereço http ou https.",
    no_types: "Escolha pelo menos um tipo de arquivo.",
    robots_blocked: "O robots.txt deste site pede para os robôs não lerem esta página. Desative Respeitar o robots.txt para lê-la mesmo assim.",
    start_page: "Não foi possível ler a página inicial: {error}",
    not_html: "O endereço inicial não é uma página web. Adicione-o como download.",
  },
};

export const feeds: Messages["feeds"] = {
  open: "Feeds",
  eyebrow: "Feeds",
  title: "Feeds RSS e Atom",
  intro: "O Downpipe verifica cada feed no seu intervalo e adiciona os arquivos dos itens novos à fila escolhida. Cada item é adicionado uma única vez, mesmo que você remova o download dele.",
  empty: "Nenhum feed ainda. Adicione um podcast, um feed de lançamentos ou qualquer outro endereço RSS ou Atom.",
  add: "Adicionar um feed",
  checkNow: "Verificar agora",
  edit: "Editar",
  off: "Desativado",
  checked: "Verificado em {date}",
  notChecked: "Ainda não verificado",
  error: "A última verificação falhou: {error}",
  addedTotal: {
    one: "{count} download adicionado no total",
    many: "{count} downloads adicionados no total",
    other: "{count} downloads adicionados no total",
  },
  checkResult: {
    none: "Nenhum item novo em {name}.",
    added: {
      one: "{count} download adicionado de {name}.",
      many: "{count} downloads adicionados de {name}.",
      other: "{count} downloads adicionados de {name}.",
    },
  },
  confirmRemove: "Remover {name}? Os downloads que ele adicionou continuam na lista.",
  form: {
    newTitle: "Novo feed",
    editTitle: "Editar feed",
    url: "Endereço do feed",
    types: "Tipos de arquivo",
    typesHint: "Vazio: qualquer arquivo que os itens ofereçam. Por exemplo mp3, m4a.",
    pattern: "Filtro de título",
    patternHint: "Uma expressão regular que o título deve conter, sem diferenciar maiúsculas, por exemplo episódio|bônus. Vazio: qualquer título.",
    interval: "Verificar a cada",
    queue: "Fila",
    enabled: "Verificar este feed",
    enabledHint: "Desativado: o feed fica na lista, mas não é verificado.",
    existing: "Baixar o que o feed tem agora",
    existingHint: "Desativado: só os itens publicados a partir de agora são baixados.",
    save: "Salvar",
    subscribe: "Adicionar feed",
    subscribed: {
      one: "Feed adicionado. {count} download adicionado.",
      many: "Feed adicionado. {count} downloads adicionados.",
      other: "Feed adicionado. {count} downloads adicionados.",
    },
    subscribedNone: "Feed adicionado. Os itens novos serão baixados a partir de agora.",
  },
  interval: {
    minutes: { one: "{count} minuto", many: "{count} minutos", other: "{count} minutos" },
    hours: { one: "{count} hora", many: "{count} horas", other: "{count} horas" },
    days: { one: "{count} dia", many: "{count} dias", other: "{count} dias" },
  },
  errors: {
    duplicate_feed: "Este feed já está na lista.",
    not_a_feed: "Esse endereço é uma página web, não um feed RSS ou Atom.",
    invalid_url: "Digite um endereço http ou https.",
  },
};

export const hosts: Messages["hosts"] = {
  privateFile: "O arquivo não está compartilhado publicamente. Peça ao dono para compartilhá-lo com qualquer pessoa que tenha o link, ou baixe-o no navegador com sua conta conectada.",
};
