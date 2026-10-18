// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are. The popup's switch
// labels quoted here match fr/popup.ts.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "Filtres · Downpipe",
  headLabel: "Extension de navigateur",
  lede:
    "Choisissez les téléchargements que le navigateur transmet à Downpipe lorsque Capturer les téléchargements du navigateur est activé, et les médias que l'extension liste lorsque Médias sur cette page est activé. Tout le reste est géré par le navigateur, comme d'habitude.",
  captureOff:
    "La capture est désactivée. Ces filtres s'appliqueront dès que vous activerez Capturer les téléchargements du navigateur dans la fenêtre de Downpipe de la barre d'outils.",
  loadError: "Impossible de charger les paramètres. Rechargez cette page pour réessayer.",
  typesHeading: "01 — Types de fichiers",
  captureAll: "Capturer tous les types de fichiers",
  captureAllDetail: "Tout téléchargement lancé par le navigateur va à Downpipe, quel que soit son type.",
  typesLabel: "Uniquement ces types de fichiers",
  typesHelp:
    "Extensions séparées par des espaces ou des virgules, comme {example}. Un téléchargement est capturé lorsque son nom de fichier se termine par l'une d'elles ; {tarGz} fonctionne aussi.",
  restoreDefault: "Restaurer la liste par défaut",
  sitesHeading: "02 — Sites exclus",
  sitesLabel: "Ne jamais capturer ni lister de médias sur ces sites",
  sitesHelp:
    "Un site par ligne. {wildcard} couvre aussi tous les sous-domaines. Un téléchargement reste dans le navigateur lorsqu'il provient de l'un de ces sites ou démarre sur l'une de leurs pages, et les médias qu'ils lisent ne sont pas listés. Les ports et les chemins sont ignorés.",
  sizeHeading: "03 — Taille minimale",
  sizeLabel: "Laisser les fichiers plus petits au navigateur",
  sizeHelp: "0 désactive cette option. Lorsque le serveur n'indique pas la taille d'un fichier, celui-ci va à Downpipe.",
  mediaHeading: "04 — Médias sur les pages",
  mediaHelp:
    "Utilisé lorsque Médias sur cette page est activé dans la fenêtre de la barre d'outils. Les flux HLS et DASH sont toujours listés ; les sites exclus ci-dessus s'appliquent aussi ici.",
  overlay: "Afficher un bouton Télécharger sur les vidéos",
  overlayDetail: "Sur une vidéo en cours de lecture, pendant quelques secondes et tant que le pointeur la survole.",
  mediaTypesLabel: "Lister les fichiers multimédias de ces types",
  mediaTypesHelp:
    "Extensions séparées par des espaces ou des virgules. Un fichier sans extension est jugé d'après le type envoyé par le serveur. Une liste vide ne liste que les flux.",
  mediaSizeLabel: "Exclure les fichiers multimédias de moins de",
  mediaSizeHelp:
    "Écarte les sons de notification et les courts aperçus. 0 liste toutes les tailles ; les fichiers dont le serveur n'indique pas la taille sont toujours listés.",
  appHeading: "05 — Application de bureau",
  startApp: "Lancer Downpipe s'il est fermé",
  startAppDetail:
    "Un téléchargement envoyé depuis le navigateur lance d'abord Downpipe dans la zone de notification, au lieu d'échouer. Si cette option est désactivée, Downpipe doit être ouvert.",
  save: "Enregistrer",
  moreErrors: { one: "Et {count} autre.", many: "Et {count} autres.", other: "Et {count} autres." },
  unsaved: "Modifications non enregistrées",
  notSavedFix: "Non enregistré. Corrigez les champs signalés en rouge.",
  saving: "Enregistrement…",
  notSavedNoAnswer: "Non enregistré. L'extension n'a pas répondu ; réessayez.",
  saved: "Enregistré",
  errors: {
    notExtension: "« {value} » n'est pas une extension de fichier.",
    tooManyTypes: {
      one: "Indiquez au plus {count} type de fichier.",
      many: "Indiquez au plus {count} types de fichiers.",
      other: "Indiquez au plus {count} types de fichiers.",
    },
    notSite: "Ligne {line} : « {value} » n'est pas un site. Utilisez un hôte comme example.com ou *.example.com.",
    tooManySites: {
      one: "Indiquez au plus {count} site.",
      many: "Indiquez au plus {count} sites.",
      other: "Indiquez au plus {count} sites.",
    },
    // The field only accepts a dot as the decimal mark, so "1.5" stays as written.
    sizeNotNumber: "Saisissez un nombre comme 2 ou 1.5, ou 0 pour désactiver cette option.",
    sizeTooBig: "Saisissez une taille inférieure à 1 TiB.",
    noTypes: "Ajoutez au moins un type de fichier, ou activez Capturer tous les types de fichiers.",
  },
};
