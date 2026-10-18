// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "Novo download",
  title: "Adicionar um link",
  titleMany: { one: "Adicionar {count} link", many: "Adicionar {count} links", other: "Adicionar {count} links" },
  addPaused: "Adicionar pausado",
  submit: {
    next: "Avançar",
    addToList: "Adicionar à lista",
    download: "Baixar",
    downloadMany: { one: "Baixar {count} arquivo", many: "Baixar {count} arquivos", other: "Baixar {count} arquivos" },
  },
  links: {
    label: "Links",
    // The pattern example stays as written: it is the syntax the field accepts.
    hint: "Um link por linha ou um padrão como file[001-100].zip. Somente http e https são aceitos.",
    found: {
      one: "{count} link encontrado. Um download para cada.",
      many: "{count} links encontrados. Um download para cada.",
      other: "{count} links encontrados. Um download para cada.",
    },
    fromPattern: {
      one: "{count} link gerado pelo padrão. Um download para cada.",
      many: "{count} links gerados pelo padrão. Um download para cada.",
      other: "{count} links gerados pelo padrão. Um download para cada.",
    },
    patternCapped: {
      one: "Somente o primeiro {count} link do padrão é usado.",
      many: "Somente os primeiros {count} links do padrão são usados.",
      other: "Somente os primeiros {count} links do padrão são usados.",
    },
    video: "Um vídeo do {site}. Você escolhe a qualidade a seguir.",
    allVideos: "Você escolhe a qualidade de cada vídeo a seguir.",
    someVideos: {
      one: "Um deles é um vídeo: você escolhe a qualidade dele a seguir.",
      many: "{count} deles são vídeos: você escolhe a qualidade deles a seguir.",
      other: "{count} deles são vídeos: você escolhe a qualidade deles a seguir.",
    },
    importList: "Importar uma lista",
    importHint: "Um arquivo .txt com um link por linha. Você também pode soltá-lo aqui.",
  },
  saveTo: {
    label: "Salvar em",
    automatic: "Automático: a pasta da categoria do arquivo.",
  },
  fileName: {
    label: "Nome do arquivo",
    placeholder: "Automático",
    hint: "Deixe vazio para usar o nome enviado pelo servidor.",
    videoHint: "Deixe vazio para usar o título do vídeo.",
    singleOnly: "Somente para um único link.",
  },
  connections: {
    label: "Conexões",
    hint: "Limite máximo",
    videoHint: "Não se aplica a vídeos",
  },
  queue: {
    label: "Fila",
    hint: "Cada fila executa os próprios downloads simultaneamente.",
  },
  checksum: {
    label: "Soma de verificação",
    // The sha256 example stays as written: it is the format the field accepts.
    hint: "Opcional, por exemplo, sha256:9f86d0… O arquivo só recebe o nome final se a soma corresponder.",
    videoHint: "Somente para arquivos: o yt-dlp monta o arquivo do vídeo a partir dos formatos do site.",
  },
  /** Sent with every request, kept in memory only. */
  headers: {
    label: "Cabeçalhos da requisição",
    hint: "Opcional. Um por linha, como Nome: valor. Enviados em cada requisição e mantidos só na memória, nunca salvos.",
  },
  /** "Paste as cURL": a command from the browser's developer tools. */
  curl: {
    button: "Colar como cURL",
    label: "Comando cURL",
    hint: "Nas ferramentas de desenvolvedor do navegador: Rede, clique com o botão direito na requisição, Copiar como cURL. Só requisições GET podem ser baixadas.",
    apply: "Preencher",
    filled: "Preenchido a partir do comando: o link e seus cabeçalhos.",
    ignored: "Deixado de fora: {list}.",
    notCurl: "Isto não é um comando cURL. Ele precisa começar com curl.",
    quote: "Uma aspa do comando nunca é fechada. Copie o comando inteiro de novo.",
    noUrl: "O comando não tem nenhum link http ou https.",
    method: "Esta requisição usa {method}. O Downpipe só baixa com GET, então não consegue repeti-la.",
    data: "Esta requisição envia dados ({option}), o que a torna um POST. O Downpipe só baixa com GET.",
  },
  errors: {
    listTooLarge: "Esse arquivo é grande demais para uma lista de links.",
    noLinks: "Cole pelo menos um link http ou https.",
    forLink: "{url}: {error}",
    headerFormat: "Linha {line}: escreva como Nome: valor.",
    headerName: "Linha {line}: {name} não é um nome de cabeçalho válido.",
    headerManaged: "Linha {line}: o Downpipe define {name} sozinho.",
  },
};
