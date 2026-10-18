// The settings dialog, one group per tab, and the browser bridge's states.
// {placeholders} that hold an element (a path, a command) are put in by the dialog.
import type { Messages } from "../core";

export const settings: Messages["settings"] = {
  eyebrow: "Configurações",
  title: "Preferências",
  tabsLabel: "Seções das configurações",
  tabs: {
    downloads: "Downloads",
    queues: "Filas",
    categories: "Categorias",
    network: "Rede",
    media: "Mídia",
    schedule: "Agendamento",
    app: "Aplicativo",
  },
  downloads: {
    folder: "Pasta padrão",
    folderHint: "As categorias abaixo criam suas pastas dentro desta.",
    concurrent: "Downloads simultâneos",
    concurrentHint: "Os outros aguardam na fila. De 1 a 16.",
    connections: "Conexões por download",
    connectionsHint: "Limite máximo. O Downpipe começa com 2 e adiciona mais somente enquanto elas ajudam. De 1 a 32.",
    speedLimit: "Limite de velocidade",
    speedLimitHint: "Compartilhado por todos os downloads. Deixe vazio para não ter limite. Vale imediatamente.",
    noLimit: "Sem limite",
    unit: "Unidade do limite de velocidade",
  },
  queues: {
    intro:
      "Cada fila executa seu próprio número de downloads simultâneos, ao lado das outras; dentro de uma fila, os downloads começam de cima para baixo. Mova downloads entre filas e altere a ordem deles pelo menu do botão direito. Os downloads de uma fila removida vão para a fila principal.",
    label: "Filas",
    name: "Nome da fila",
    namePlaceholder: "Nome",
    // The main queue's limit, which is set on the Downloads tab; "por vez" does not change with the number.
    mainLimit: { one: "{count} por vez", many: "{count} por vez", other: "{count} por vez" },
    mainLimitTitle: "Definido em Downloads, Downloads simultâneos",
    limit: "Downloads simultâneos de {name}",
    limitUnnamed: "Downloads simultâneos da fila",
    limitTitle: "Downloads simultâneos, de 1 a 16",
    remove: "Remover {name}",
    removeUnnamed: "Remover fila",
    newName: "Fila {number}",
    add: "Adicionar fila",
  },
  categories: {
    use: "Organizar downloads em pastas por categoria",
    useHint:
      "Um novo download vai para a pasta da sua categoria conforme o tipo de arquivo. Quando o nome de arquivo enviado pelo servidor indica outro tipo, o arquivo concluído é movido para a pasta certa. As pastas que você mesmo escolhe nunca são alteradas.",
    label: "Categorias",
    name: "Nome da categoria",
    namePlaceholder: "Nome",
    types: "Tipos de arquivo de {name}",
    typesUnnamed: "Tipos de arquivo da categoria",
    remove: "Remover {name}",
    removeUnnamed: "Remover categoria",
    folder: "Pasta de {name}",
    folderUnnamed: "Pasta da categoria",
    folderPlaceholder: "Pasta: {name} dentro da pasta padrão",
    folderPlaceholderUnnamed: "Pasta: nome dentro da pasta padrão",
    add: "Adicionar categoria",
  },
  network: {
    proxy: "Proxy",
    proxyHint:
      "http://, https:// ou socks5://host:port. Vazio usa o proxy do sistema. Vale para os downloads iniciados depois de salvar.",
    proxyPlaceholder: "Proxy do sistema",
    userAgent: "Agente do usuário",
    userAgentHint: "Alguns servidores só respondem a navegadores. Vazio envia o do próprio Downpipe.",
  },
  media: {
    ffmpegHint:
      "Junta o vídeo e o som separados de uma transmissão em um só arquivo, sem recodificar. Vazio usa o ffmpeg do PATH. O Downpipe não inclui o FFmpeg.",
    ffmpegPlaceholder: "ffmpeg no PATH",
    found: "Encontrado",
    notFound: "Não encontrado",
    ffmpegMissing:
      "Transmissões cujo vídeo e som vêm separados são salvas como dois arquivos. Instale o FFmpeg e adicione-o ao PATH, ou informe o caminho do ffmpeg acima e salve.",
    streams:
      "As transmissões vêm da detecção de mídia da extensão do navegador ou de links .m3u8 e .mpd adicionados aqui. Transmissões protegidas por DRM e ao vivo não são baixadas.",
  },
  siteTools: {
    template: "Modelo de nome dos arquivos de mídia",
    templateHint: "Dá nome aos vídeos de páginas como o YouTube; a extensão é adicionada. Clique em um marcador para inseri-lo.",
    templatePreview: "Exemplo: {name}",
    templateSame: "Sem o título nem o id, todos os vídeos recebem o mesmo nome, e o Downpipe acrescenta (1), (2) aos seguintes.",
    templateInsert: "Marcadores",
    placeholders: {
      title: "O título do vídeo",
      channel: "O nome do canal",
      date: "A data de envio, como 2005-04-23",
      index: "A posição na playlist, como 07 de 42",
      id: "O id do vídeo no site",
    },
    heading: "Vídeos em páginas",
    // The extension's menu item is only in English, so its name stays as the user sees it.
    intro:
      "Em uma página do YouTube, o comando Baixar com o Downpipe entrega a página ao yt-dlp, que extrai o vídeo. O yt-dlp precisa do Deno para o YouTube e do FFmpeg para juntar vídeo e som acima de 360p.",
    ytDlpHint: "Vazio usa a cópia baixada abaixo; se não houver, o yt-dlp do PATH.",
    ytDlpPlaceholder: "Cópia baixada ou yt-dlp no PATH",
    denoHint:
      "Versão 2.3.0 ou mais recente. Vazio usa a cópia baixada abaixo; se não houver, o deno do PATH; se não houver, o Node 22 ou mais recente.",
    denoPlaceholder: "Cópia baixada ou deno no PATH",
    looking: "Procurando o yt-dlp e o Deno…",
    listLabel: "Ferramentas para vídeos em páginas",
    downloading: "Baixando…",
    // {folder} is a path, or toolsFolder below when the path is not known yet.
    installNote:
      "Somente quando você clica: as compilações oficiais de github.com/yt-dlp/yt-dlp e github.com/denoland/deno, cada uma verificada com o SHA-256 publicado no respectivo lançamento, vão para {folder}. O YouTube muda com frequência; clique novamente para obter o yt-dlp mais recente.",
    toolsFolder: "a pasta de ferramentas do Downpipe",
    cookies:
      "O Downpipe não envia cookies ao yt-dlp, então vídeos privados, exclusivos para membros e com restrição de idade não são baixados, nem os protegidos (DRM) e ao vivo. Cumprir os termos do YouTube é responsabilidade sua.",
  },
  schedule: {
    enable: "Iniciar e parar downloads em horários agendados",
    enableHint:
      "No horário de início, os downloads pausados e com falha são retomados. No horário de parada, eles são pausados. O Downpipe precisa estar em execução, na janela ou na bandeja do sistema.",
    queue: "Fila",
    queueHint: "As outras filas não são afetadas.",
    everyQueue: "Todas as filas",
    start: "Iniciar às",
    stop: "Parar às",
    stopHint: "Opcional",
    days: "Dias",
  },
  app: {
    language: "Idioma",
    languageHint: "Aplica-se na hora à janela, ao menu da bandeja do sistema e às notificações.",
    systemDefault: "Padrão do sistema ({language})",
    notify: "Notificar quando um download terminar",
    notifyHint: "Uma notificação do sistema quando um download é concluído ou falha enquanto a janela não está em primeiro plano.",
    tray: "Continuar em execução na bandeja do sistema ao fechar a janela",
    trayHint: "Os downloads continuam, e a extensão do navegador ainda pode enviar links. Para sair, use o ícone na bandeja do sistema.",
    startup: "Iniciar o Downpipe ao entrar na sua conta",
    startupHint: "Inicia na bandeja do sistema, para que os downloads capturados sempre tenham para onde ir.",
    clipboard: "Oferecer o download de links copiados",
    clipboardHint:
      "Quando você copia um link para um tipo de arquivo listado em Categorias, ou para um vídeo do YouTube, o Downpipe o abre na caixa de diálogo Adicionar um link. Links que já estão na lista são ignorados.",
    awake: "Manter o computador ativo durante os downloads",
    awakeHint: "A suspensão aguarda até que os downloads parem. A tela ainda pode desligar.",
    confirmBrowser: "Perguntar antes de iniciar downloads do navegador",
    confirmBrowserHint:
      "Escolha antes o nome do arquivo, a pasta e a fila, em uma pequena janela sobre o navegador. Desativado: eles começam na hora, com a pasta automática.",
  },
  bridge: {
    heading: "Ponte com o navegador",
    register: "Registrar nos navegadores ao iniciar",
    registerHint:
      "Permite que a extensão do Downpipe no Chrome, Edge, Chromium e Firefox se comunique com esta cópia do aplicativo. Altera somente a sua conta de usuário. Desative para gerenciar o registro com o downpipe-native-host por conta própria.",
    checking: "Verificando o registro…",
    noHost:
      "Esta cópia do Downpipe não tem a ponte com o navegador em {host}. Reinstale o Downpipe; em uma compilação de desenvolvimento, execute {command}.",
    listLabel: "Registro por navegador",
    registering: "Registrando…",
    registerNow: "Registrar agora",
    pickUp: "Os navegadores abertos detectam a mudança na próxima vez que a extensão verificar.",
    states: {
      registered: "Registrado",
      notInstalled: "Não instalado",
      missing: "Não registrado",
      invalid: "Ilegível",
      otherHost: "Outra cópia",
      extensionNotAllowed: "Outra extensão",
      failed: "Erro",
    },
  },
};
