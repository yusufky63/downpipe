// Trouver des fichiers sur le web : l'aspirateur de site, les abonnements aux flux et les hébergeurs de fichiers.
import type { Messages } from "../core";

export const grabber: Messages["grabber"] = {
  open: "Aspirateur de site",
  eyebrow: "Aspirateur de site",
  title: "Trouver des fichiers sur un site",
  address: {
    label: "Adresse de départ",
    hint: "Downpipe lit cette page et suit ses liens. Aucun cookie n'est envoyé : il voit ce que voit un visiteur non connecté.",
  },
  depth: {
    label: "Profondeur",
    hint: "1 ne lit que cette page ; chaque niveau suit un niveau de liens de plus.",
  },
  pages: {
    label: "Limite de pages",
    hint: "Pages lues au maximum",
  },
  types: {
    label: "Types de fichiers",
    custom: "Autres types",
    customPlaceholder: "p. ex. iso, torrent",
  },
  sameSite: {
    label: "Rester sur ce site",
    hint: "Seules les pages du site de l'adresse de départ sont lues. Les fichiers liés depuis d'autres sites sont tout de même listés.",
  },
  robots: {
    label: "Respecter robots.txt",
    hint: "Ignore les pages qu'un site demande aux robots d'éviter.",
  },
  sizes: {
    label: "Obtenir la taille des fichiers",
    hint: "Une petite requête par fichier, pour les {count} premiers fichiers.",
  },
  delay: {
    label: "Pause",
    hint: "Millisecondes entre deux requêtes vers le même site. Les sites peuvent bloquer les visiteurs trop pressés.",
  },
  start: "Trouver les fichiers",
  stop: "Arrêter",
  again: "Nouvelle recherche",
  progress: {
    pages: { one: "{done} page lue sur {count}", many: "{done} pages lues sur {count}", other: "{done} pages lues sur {count}" },
    sizes: {
      one: "Tailles en cours : {done} fichier sur {count}",
      many: "Tailles en cours : {done} fichiers sur {count}",
      other: "Tailles en cours : {done} fichiers sur {count}",
    },
    files: { one: "{count} fichier trouvé", many: "{count} fichiers trouvés", other: "{count} fichiers trouvés" },
  },
  results: {
    title: { one: "{count} fichier trouvé", many: "{count} fichiers trouvés", other: "{count} fichiers trouvés" },
    none: "Aucun fichier de ces types n'a été trouvé.",
    canceled: "Arrêté avant la fin : voici les fichiers trouvés jusque-là.",
    pageLimit: "La limite de pages a été atteinte avant que toutes les pages soient lues.",
    blocked: {
      one: "{count} page ou fichier ignoré, car robots.txt ne l'autorise pas.",
      many: "{count} pages ou fichiers ignorés, car robots.txt ne les autorise pas.",
      other: "{count} pages ou fichiers ignorés, car robots.txt ne les autorise pas.",
    },
    failed: { one: "{count} page n'a pas pu être lue.", many: "{count} pages n'ont pas pu être lues.", other: "{count} pages n'ont pas pu être lues." },
    filter: "Filtrer les fichiers",
    selectAll: "Sélectionner tous les fichiers affichés",
    name: "Nom",
    size: "Taille",
    foundOn: "Trouvé sur",
    selected: { one: "{count} sélectionné, {size}", many: "{count} sélectionnés, {size}", other: "{count} sélectionnés, {size}" },
    unknownSizes: { one: "{count} de taille inconnue", many: "{count} de taille inconnue", other: "{count} de taille inconnue" },
    add: { one: "Ajouter {count} fichier", many: "Ajouter {count} fichiers", other: "Ajouter {count} fichiers" },
    added: { one: "{count} fichier ajouté.", many: "{count} fichiers ajoutés.", other: "{count} fichiers ajoutés." },
    alreadyListed: { one: "{count} était déjà dans la liste.", many: "{count} étaient déjà dans la liste.", other: "{count} étaient déjà dans la liste." },
  },
  errors: {
    invalid_url: "Saisissez une adresse http ou https.",
    no_types: "Choisissez au moins un type de fichier.",
    robots_blocked: "Le robots.txt de ce site demande aux robots de ne pas lire cette page. Désactivez Respecter robots.txt pour la lire quand même.",
    start_page: "La page de départ n'a pas pu être lue : {error}",
    not_html: "L'adresse de départ n'est pas une page web. Ajoutez-la plutôt comme téléchargement.",
  },
};

export const feeds: Messages["feeds"] = {
  open: "Flux",
  eyebrow: "Flux",
  title: "Flux RSS et Atom",
  intro: "Downpipe vérifie chaque flux à son intervalle et ajoute les fichiers des nouveaux éléments à la file choisie. Un élément n'est ajouté qu'une fois, même si vous retirez son téléchargement.",
  empty: "Aucun flux pour l'instant. Ajoutez un podcast, un flux de versions ou toute autre adresse RSS ou Atom.",
  add: "Ajouter un flux",
  checkNow: "Vérifier maintenant",
  edit: "Modifier",
  off: "Désactivé",
  checked: "Vérifié le {date}",
  notChecked: "Pas encore vérifié",
  error: "La dernière vérification a échoué : {error}",
  addedTotal: {
    one: "{count} téléchargement ajouté au total",
    many: "{count} téléchargements ajoutés au total",
    other: "{count} téléchargements ajoutés au total",
  },
  checkResult: {
    none: "Aucun nouvel élément dans {name}.",
    added: {
      one: "{count} téléchargement ajouté depuis {name}.",
      many: "{count} téléchargements ajoutés depuis {name}.",
      other: "{count} téléchargements ajoutés depuis {name}.",
    },
  },
  confirmRemove: "Retirer {name} ? Les téléchargements qu'il a ajoutés restent dans la liste.",
  form: {
    newTitle: "Nouveau flux",
    editTitle: "Modifier le flux",
    url: "Adresse du flux",
    types: "Types de fichiers",
    typesHint: "Vide : tout fichier proposé par les éléments. Par exemple mp3, m4a.",
    pattern: "Filtre de titre",
    patternHint: "Une expression régulière que le titre doit contenir, sans tenir compte de la casse, par exemple épisode|bonus. Vide : tout titre.",
    interval: "Vérifier toutes les",
    queue: "File",
    enabled: "Vérifier ce flux",
    enabledHint: "Désactivé : le flux reste dans la liste mais n'est pas vérifié.",
    existing: "Télécharger le contenu actuel",
    existingHint: "Désactivé : seuls les éléments publiés à partir de maintenant sont téléchargés.",
    save: "Enregistrer",
    subscribe: "Ajouter le flux",
    subscribed: {
      one: "Flux ajouté. {count} téléchargement ajouté.",
      many: "Flux ajouté. {count} téléchargements ajoutés.",
      other: "Flux ajouté. {count} téléchargements ajoutés.",
    },
    subscribedNone: "Flux ajouté. Les nouveaux éléments seront téléchargés à partir de maintenant.",
  },
  interval: {
    minutes: { one: "{count} minute", many: "{count} minutes", other: "{count} minutes" },
    hours: { one: "{count} heure", many: "{count} heures", other: "{count} heures" },
    days: { one: "{count} jour", many: "{count} jours", other: "{count} jours" },
  },
  errors: {
    duplicate_feed: "Ce flux est déjà dans la liste.",
    not_a_feed: "Cette adresse est une page web, pas un flux RSS ou Atom.",
    invalid_url: "Saisissez une adresse http ou https.",
  },
};
