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
  /** Sent with every request, kept in memory only. */
  headers: {
    label: "İstek başlıkları",
    hint: "İsteğe bağlı. Her satıra bir tane, Ad: değer biçiminde. Her istekle gönderilir; yalnızca bellekte tutulur, asla kaydedilmez.",
  },
  /** "Paste as cURL": a command from the browser's developer tools. */
  curl: {
    button: "cURL olarak yapıştır",
    label: "cURL komutu",
    hint: "Tarayıcının geliştirici araçlarında: Ağ, isteğe sağ tıklayın, cURL olarak kopyala. Yalnızca GET istekleri indirilebilir.",
    apply: "Doldur",
    filled: "Komuttan dolduruldu: bağlantı ve başlıkları.",
    ignored: "Alınmayanlar: {list}.",
    notCurl: "Bu bir cURL komutu değil. curl ile başlamalı.",
    quote: "Komuttaki bir tırnak hiç kapanmıyor. Komutun tamamını yeniden kopyalayın.",
    noUrl: "Komutta http veya https bağlantısı yok.",
    method: "Bu istek {method} kullanıyor. Downpipe yalnızca GET ile indirir, bu yüzden bu isteği tekrarlayamaz.",
    data: "Bu istek veri gönderiyor ({option}), bu da onu bir POST yapar. Downpipe yalnızca GET ile indirir.",
  },
  errors: {
    listTooLarge: "Bu dosya bir bağlantı listesi için çok büyük.",
    noLinks: "En az bir http ya da https bağlantısı yapıştırın.",
    forLink: "{url}: {error}",
    headerFormat: "Satır {line}: Ad: değer biçiminde yazın.",
    headerName: "Satır {line}: {name} geçerli bir başlık adı değil.",
    headerManaged: "Satır {line}: {name} başlığını Downpipe kendisi ayarlar.",
  },
};
