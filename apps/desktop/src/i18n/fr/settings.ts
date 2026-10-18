// The settings dialog, one group per tab, and the browser bridge's states.
// {placeholders} that hold an element (a path, a command) are put in by the dialog.
import type { Messages } from "../core";

export const settings: Messages["settings"] = {
  eyebrow: "Paramètres",
  title: "Préférences",
  tabsLabel: "Sections des paramètres",
  tabs: {
    downloads: "Téléchargements",
    queues: "Files d'attente",
    categories: "Catégories",
    network: "Réseau",
    sites: "Sites",
    media: "Médias",
    schedule: "Planification",
    app: "Application",
  },
  downloads: {
    folder: "Dossier par défaut",
    folderHint: "Les catégories ci-dessous créent leurs dossiers dans celui-ci.",
    concurrent: "Téléchargements simultanés",
    concurrentHint: "Les autres attendent dans la file d'attente. De 1 à 16.",
    connections: "Connexions par téléchargement",
    connectionsHint: "Limite supérieure. Downpipe commence avec 2 et n'en ajoute que tant qu'elles sont utiles. De 1 à 32.",
    speedLimit: "Limite de vitesse",
    speedLimitHint: "Partagée par tous les téléchargements. Laissez vide pour ne pas limiter. S'applique immédiatement.",
    speedLimitInvalid: "Saisissez un nombre, par exemple 1,5 ou 1.5.",
    noLimit: "Aucune limite",
    unit: "Unité de la limite de vitesse",
    duplicate: "Quand un lien est déjà dans la liste",
    duplicateHint: "\"Ne plus demander\" dans la question règle aussi ce choix.",
    duplicateAsk: "Demander à chaque fois",
  },
  queues: {
    intro:
      "Chaque file d'attente exécute son propre nombre de téléchargements simultanés, en parallèle des autres ; dans une file d'attente, les téléchargements démarrent de haut en bas. Déplacez les téléchargements d'une file d'attente à l'autre et changez leur ordre depuis le menu contextuel. Les téléchargements d'une file d'attente supprimée passent dans la file d'attente principale.",
    label: "Files d'attente",
    name: "Nom de la file d'attente",
    namePlaceholder: "Nom",
    mainLimit: { one: "{count} à la fois", many: "{count} à la fois", other: "{count} à la fois" },
    mainLimitTitle: "Défini dans Téléchargements, Téléchargements simultanés",
    limit: "Téléchargements simultanés de {name}",
    limitUnnamed: "Téléchargements simultanés de la file d'attente",
    limitTitle: "Téléchargements simultanés, de 1 à 16",
    remove: "Supprimer {name}",
    removeUnnamed: "Supprimer la file d'attente",
    newName: "File d'attente {number}",
    add: "Ajouter une file d'attente",
  },
  categories: {
    use: "Trier les téléchargements dans des dossiers par catégorie",
    useHint:
      "Un nouveau téléchargement va dans le dossier de sa catégorie, selon le type de fichier. Lorsque le nom de fichier envoyé par le serveur indique un autre type, le fichier terminé est déplacé dans le bon dossier. Les dossiers que vous choisissez vous-même ne sont jamais modifiés.",
    label: "Catégories",
    name: "Nom de la catégorie",
    namePlaceholder: "Nom",
    types: "Types de fichiers de {name}",
    typesUnnamed: "Types de fichiers de la catégorie",
    remove: "Supprimer {name}",
    removeUnnamed: "Supprimer la catégorie",
    folder: "Dossier de {name}",
    folderUnnamed: "Dossier de la catégorie",
    folderPlaceholder: "Dossier : {name} dans le dossier par défaut",
    folderPlaceholderUnnamed: "Dossier : nom dans le dossier par défaut",
    add: "Ajouter une catégorie",
  },
  network: {
    proxy: "Proxy",
    proxyHint:
      "http://, https:// ou socks5://hôte:port. Laissez vide pour utiliser le proxy système. S'applique aux téléchargements lancés après l'enregistrement.",
    proxyPlaceholder: "Proxy système",
    userAgent: "Agent utilisateur",
    userAgentHint: "Certains serveurs ne répondent qu'aux navigateurs. Laissez vide pour envoyer celui de Downpipe.",
  },
  media: {
    ffmpegHint:
      "Fusionne la vidéo et le son séparés d'un flux en un seul fichier, sans réencodage. Laissez vide pour utiliser ffmpeg du PATH. Downpipe n'inclut pas FFmpeg.",
    ffmpegPlaceholder: "ffmpeg dans le PATH",
    found: "Trouvé",
    notFound: "Introuvable",
    ffmpegMissing:
      "Les flux dont la vidéo et le son arrivent séparément sont enregistrés en deux fichiers. Installez FFmpeg et ajoutez-le au PATH, ou saisissez le chemin de ffmpeg ci-dessus, puis enregistrez.",
    streams:
      "Les flux proviennent de la détection de médias de l'extension de navigateur, ou des liens .m3u8 et .mpd ajoutés ici. Les flux protégés par DRM et les flux en direct ne sont pas téléchargés.",
  },
  siteTools: {
    template: "Modèle de nom des fichiers multimédias",
    templateHint: "Nomme les vidéos de pages comme YouTube ; l'extension est ajoutée. Cliquez sur un champ pour l'insérer.",
    templatePreview: "Exemple : {name}",
    templateSame: "Sans le titre ni l'identifiant, toutes les vidéos portent le même nom, et Downpipe ajoute (1), (2) aux suivantes.",
    templateInsert: "Champs",
    placeholders: {
      title: "Le titre de la vidéo",
      channel: "Le nom de la chaîne",
      date: "La date de mise en ligne, comme 2005-04-23",
      index: "La position dans la playlist, comme 07 sur 42",
      id: "L'identifiant de la vidéo sur le site",
    },
    heading: "Vidéos sur les pages web",
    // Quotes the extension's French menu item (browser-extension fr/menus.ts, menus.link).
    intro:
      "Sur une page YouTube, la commande « Télécharger avec Downpipe » transmet la page à yt-dlp, qui en extrait la vidéo. yt-dlp a besoin de Deno pour YouTube, et de FFmpeg pour assembler la vidéo et le son au-delà de 360p.",
    ytDlpHint: "Laissez vide pour utiliser la copie téléchargée ci-dessous, sinon yt-dlp du PATH.",
    ytDlpPlaceholder: "Copie téléchargée, sinon yt-dlp dans le PATH",
    denoHint:
      "Version 2.3.0 ou ultérieure. Laissez vide pour utiliser la copie téléchargée ci-dessous, sinon deno du PATH, sinon Node 22 ou ultérieur.",
    denoPlaceholder: "Copie téléchargée, sinon deno dans le PATH",
    looking: "Recherche de yt-dlp et Deno…",
    listLabel: "Outils pour les vidéos sur les pages web",
    downloading: "Téléchargement…",
    installNote:
      "Uniquement lorsque vous cliquez sur ce bouton : les versions officielles de github.com/yt-dlp/yt-dlp et github.com/denoland/deno, chacune vérifiée avec l'empreinte SHA-256 publiée pour sa version, sont placées dans {folder}. YouTube change souvent ; cliquez de nouveau pour obtenir la dernière version de yt-dlp.",
    toolsFolder: "le dossier des outils de Downpipe",
    cookies:
      "Downpipe n'envoie aucun cookie à yt-dlp : les vidéos privées, réservées aux membres ou soumises à une limite d'âge ne sont donc pas téléchargées, pas plus que les vidéos protégées (DRM) et en direct. Le respect des conditions d'utilisation de YouTube relève de votre responsabilité.",
  },
  schedule: {
    enable: "Démarrer et arrêter les téléchargements selon une planification",
    enableHint:
      "À l'heure de début, les téléchargements suspendus et en échec reprennent. À l'heure de fin, ils sont suspendus. Downpipe doit être en cours d'exécution, dans la fenêtre ou dans la zone de notification.",
    queue: "File d'attente",
    queueHint: "Les autres files d'attente ne sont pas concernées.",
    everyQueue: "Toutes les files d'attente",
    start: "Démarrer à",
    stop: "Arrêter à",
    stopHint: "Facultatif",
    days: "Jours",
  },
  app: {
    language: "Langue",
    languageHint: "S'applique immédiatement à la fenêtre, au menu de la zone de notification et aux notifications.",
    systemDefault: "Valeur par défaut du système ({language})",
    notify: "Avertir lorsqu'un téléchargement se termine",
    notifyHint: "Une notification système lorsqu'un téléchargement se termine ou échoue alors que la fenêtre n'est pas au premier plan.",
    tray: "Continuer à s'exécuter dans la zone de notification à la fermeture de la fenêtre",
    trayHint:
      "Les téléchargements continuent, et l'extension de navigateur peut toujours envoyer des liens. Pour quitter, utilisez l'icône de la zone de notification.",
    startup: "Démarrer Downpipe à l'ouverture de session",
    startupHint: "Démarre dans la zone de notification, pour que les téléchargements capturés aient toujours une destination.",
    clipboard: "Proposer de télécharger les liens copiés",
    clipboardHint:
      "Lorsque vous copiez un lien vers un type de fichier répertorié dans Catégories, ou vers une vidéo YouTube, Downpipe ouvre la boîte de dialogue d'ajout avec ce lien. Les liens déjà présents dans la liste sont ignorés.",
    awake: "Empêcher la mise en veille de l'ordinateur pendant les téléchargements",
    awakeHint: "La mise en veille attend qu'aucun téléchargement ne soit plus en cours. L'écran peut toujours s'éteindre.",
    confirmBrowser: "Demander avant de démarrer les téléchargements venant du navigateur",
    confirmBrowserHint:
      "Choisissez d'abord le nom du fichier, le dossier et la file d'attente, dans une petite fenêtre au-dessus du navigateur. Désactivé : ils démarrent aussitôt avec le dossier automatique.",
  },
  bridge: {
    heading: "Passerelle de navigateur",
    register: "S'enregistrer auprès des navigateurs au démarrage",
    registerHint:
      "Permet à l'extension Downpipe de Chrome, Edge, Chromium et Firefox de communiquer avec cette copie de l'application. Ne modifie que votre compte utilisateur. Désactivez cette option pour gérer vous-même l'enregistrement avec downpipe-native-host.",
    checking: "Vérification de l'enregistrement…",
    noHost:
      "Cette copie de Downpipe n'a pas de passerelle de navigateur à l'emplacement {host}. Réinstallez Downpipe ; dans une version de développement, exécutez {command}.",
    listLabel: "Enregistrement par navigateur",
    registering: "Enregistrement…",
    registerNow: "Enregistrer maintenant",
    pickUp: "Les navigateurs ouverts le prendront en compte la prochaine fois que l'extension vérifiera.",
    states: {
      registered: "Enregistré",
      notInstalled: "Non installé",
      missing: "Non enregistré",
      invalid: "Illisible",
      otherHost: "Autre copie",
      extensionNotAllowed: "Autre extension",
      failed: "Erreur",
    },
  },
};
