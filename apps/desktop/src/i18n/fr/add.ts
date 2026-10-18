// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "Nouveau téléchargement",
  title: "Ajouter un lien",
  titleMany: { one: "Ajouter {count} lien", many: "Ajouter {count} liens", other: "Ajouter {count} liens" },
  addPaused: "Ajouter en pause",
  submit: {
    next: "Suivant",
    addToList: "Ajouter à la liste",
    download: "Télécharger",
    downloadMany: { one: "Télécharger {count} fichier", many: "Télécharger {count} fichiers", other: "Télécharger {count} fichiers" },
  },
  links: {
    label: "Liens",
    // The pattern example stays as written: it is the syntax the field accepts.
    hint: "Un lien par ligne, ou un modèle comme file[001-100].zip. Les liens http, https et magnet sont pris en charge.",
    found: {
      one: "{count} lien trouvé. Un téléchargement par lien.",
      many: "{count} liens trouvés. Un téléchargement par lien.",
      other: "{count} liens trouvés. Un téléchargement par lien.",
    },
    fromPattern: {
      one: "{count} lien issu du modèle. Un téléchargement par lien.",
      many: "{count} liens issus du modèle. Un téléchargement par lien.",
      other: "{count} liens issus du modèle. Un téléchargement par lien.",
    },
    // French puts the number before "premiers", so the singular names no number.
    patternCapped: {
      one: "Seul le premier lien du modèle est utilisé.",
      many: "Seuls les {count} premiers liens du modèle sont utilisés.",
      other: "Seuls les {count} premiers liens du modèle sont utilisés.",
    },
    video: "Une vidéo {site}. Vous choisirez sa qualité à l'étape suivante.",
    allVideos: "Vous choisirez la qualité de chaque vidéo à l'étape suivante.",
    someVideos: {
      one: "L'un d'eux est une vidéo : vous choisirez sa qualité à l'étape suivante.",
      many: "{count} d'entre eux sont des vidéos : vous choisirez leur qualité à l'étape suivante.",
      other: "{count} d'entre eux sont des vidéos : vous choisirez leur qualité à l'étape suivante.",
    },
    importList: "Importer une liste",
    importHint: "Un fichier .txt avec un lien par ligne. Vous pouvez aussi le déposer ici.",
  },
  saveTo: {
    label: "Enregistrer dans",
    automatic: "Automatique : le dossier de la catégorie du fichier.",
  },
  fileName: {
    label: "Nom du fichier",
    placeholder: "Automatique",
    hint: "Laissez vide pour utiliser le nom envoyé par le serveur.",
    videoHint: "Laissez vide pour utiliser le titre de la vidéo.",
    singleOnly: "Uniquement pour un seul lien.",
  },
  connections: {
    label: "Connexions",
    hint: "Limite supérieure",
    videoHint: "Pas pour les vidéos",
  },
  queue: {
    label: "File d'attente",
    hint: "Chaque file d'attente exécute ses propres téléchargements simultanés.",
  },
  checksum: {
    label: "Somme de contrôle",
    // The sha256 example stays as written: it is the format the field accepts.
    hint: "Facultatif, par ex. sha256:9f86d0… Le fichier ne prend son nom définitif que s'il correspond.",
    videoHint: "Uniquement pour les fichiers : yt-dlp assemble le fichier d'une vidéo à partir des formats du site.",
  },
  /** Sent with every request, kept in memory only. */
  headers: {
    label: "En-têtes de la requête",
    hint: "Facultatif. Un par ligne, sous la forme Nom : valeur. Envoyés avec chaque requête et gardés en mémoire seulement, jamais enregistrés.",
  },
  /** "Paste as cURL": a command from the browser's developer tools. */
  curl: {
    button: "Coller comme cURL",
    label: "Commande cURL",
    hint: "Dans les outils de développement du navigateur : Réseau, clic droit sur la requête, Copier en tant que cURL. Seules les requêtes GET peuvent être téléchargées.",
    apply: "Remplir",
    filled: "Rempli à partir de la commande : le lien et ses en-têtes.",
    ignored: "Laissé de côté : {list}.",
    notCurl: "Ce n'est pas une commande cURL. Elle doit commencer par curl.",
    quote: "Un guillemet de la commande n'est jamais fermé. Copiez de nouveau la commande entière.",
    noUrl: "La commande ne contient aucun lien http ou https.",
    method: "Cette requête utilise {method}. Downpipe télécharge uniquement en GET et ne peut donc pas la répéter.",
    data: "Cette requête envoie des données ({option}), ce qui en fait un POST. Downpipe télécharge uniquement en GET.",
  },
  errors: {
    listTooLarge: "Ce fichier est trop volumineux pour une liste de liens.",
    noLinks: "Collez au moins un lien http, https ou magnet.",
    forLink: "{url} : {error}",
    headerFormat: "Ligne {line} : écrivez-le sous la forme Nom : valeur.",
    headerName: "Ligne {line} : {name} n'est pas un nom d'en-tête valide.",
    headerManaged: "Ligne {line} : Downpipe définit {name} lui-même.",
  },
};
