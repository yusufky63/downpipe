// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "Yeni indirme",
  title: "Bağlantı ekle",
  titleMany: { one: "{count} bağlantı ekle", other: "{count} bağlantı ekle" },
  addPaused: "Duraklatılmış ekle",
  submit: {
    next: "İleri",
    addToList: "Listeye ekle",
    download: "İndir",
    downloadMany: { one: "{count} dosya indir", other: "{count} dosya indir" },
  },
  links: {
    playlist: "Bir {site} oynatma listesi. Sonraki adımda videolarını seçersiniz.",
    channel: "Bir {site} kanalı. Sonraki adımda videolarını seçersiniz.",
    label: "Bağlantılar",
    hint: "Her satıra bir bağlantı ya da file[001-100].zip gibi bir desen. Yalnızca http ve https desteklenir.",
    found: { one: "{count} bağlantı bulundu. Her biri için bir indirme.", other: "{count} bağlantı bulundu. Her biri için bir indirme." },
    fromPattern: { one: "Desenden {count} bağlantı. Her biri için bir indirme.", other: "Desenden {count} bağlantı. Her biri için bir indirme." },
    patternCapped: { one: "Desenin yalnızca ilk {count} bağlantısı kullanılır.", other: "Desenin yalnızca ilk {count} bağlantısı kullanılır." },
    video: "Bir {site} videosu. Kalitesini sonraki adımda seçersiniz.",
    allVideos: "Her videonun kalitesini sonraki adımda seçersiniz.",
    someVideos: {
      one: "Bunlardan biri video: kalitesini sonraki adımda seçersiniz.",
      other: "Bunlardan {count} tanesi video: kalitelerini sonraki adımda seçersiniz.",
    },
    importList: "Liste içe aktar",
    importHint: "Her satırda bir bağlantı olan bir .txt dosyası. Buraya da bırakabilirsiniz.",
  },
  saveTo: {
    label: "Kaydetme yeri",
    automatic: "Otomatik: dosyanın kategorisinin klasörü.",
  },
  fileName: {
    label: "Dosya adı",
    placeholder: "Otomatik",
    hint: "Sunucunun gönderdiği adı kullanmak için boş bırakın.",
    videoHint: "Videonun başlığını kullanmak için boş bırakın.",
    singleOnly: "Yalnızca tek bağlantı için.",
  },
  connections: {
    label: "Bağlantı sayısı",
    hint: "Üst sınır",
    videoHint: "Videolar için değil",
  },
  queue: {
    label: "Kuyruk",
    hint: "Her kuyruk kendi indirmelerini aynı anda çalıştırır.",
  },
  checksum: {
    label: "Sağlama toplamı",
    hint: "İsteğe bağlı, ör. sha256:9f86d0… Dosya yalnızca eşleşirse yerine konur.",
    videoHint: "Yalnızca dosyalar için: yt-dlp videonun dosyasını sitenin biçimlerinden birleştirir.",
  },
  errors: {
    listTooLarge: "Bu dosya bir bağlantı listesi için çok büyük.",
    noLinks: "En az bir http ya da https bağlantısı yapıştırın.",
    forLink: "{url}: {error}",
  },
};
