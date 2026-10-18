// Web'de dosya bulma: site tarayıcı, akış abonelikleri ve dosya barındırıcıları.
import type { Messages } from "../core";

export const grabber: Messages["grabber"] = {
  open: "Site tarayıcı",
  eyebrow: "Site tarayıcı",
  title: "Bir sitede dosya bul",
  address: {
    label: "Başlangıç adresi",
    hint: "Downpipe bu sayfayı okur ve bağlantılarını izler. Çerez gönderilmez; oturum açmamış bir ziyaretçinin gördüğünü görür.",
  },
  depth: {
    label: "Derinlik",
    hint: "1 yalnızca bu sayfayı okur; her adım bir kat daha bağlantı izler.",
  },
  pages: {
    label: "Sayfa sınırı",
    hint: "En fazla okunacak sayfa",
  },
  types: {
    label: "Dosya türleri",
    custom: "Diğer türler",
    customPlaceholder: "ör. iso, torrent",
  },
  sameSite: {
    label: "Bu sitede kal",
    hint: "Yalnızca başlangıç adresinin sitesindeki sayfalar okunur. Başka sitelerden bağlanan dosyalar yine listelenir.",
  },
  robots: {
    label: "robots.txt kurallarına uy",
    hint: "Sitenin tarayıcılardan uzak durmasını istediği sayfaları atla.",
  },
  sizes: {
    label: "Dosya boyutlarını öğren",
    hint: "İlk {count} dosya için dosya başına küçük bir istek.",
  },
  delay: {
    label: "Bekleme",
    hint: "Aynı siteye iki istek arasındaki milisaniye. Siteler acele eden ziyaretçileri engelleyebilir.",
  },
  start: "Dosyaları bul",
  stop: "Durdur",
  again: "Yeni arama",
  progress: {
    pages: { one: "{count} sayfadan {done} tanesi okundu", other: "{count} sayfadan {done} tanesi okundu" },
    sizes: { one: "Boyutlar öğreniliyor: {count} dosyadan {done}", other: "Boyutlar öğreniliyor: {count} dosyadan {done}" },
    files: { one: "{count} dosya bulundu", other: "{count} dosya bulundu" },
  },
  results: {
    title: { one: "{count} dosya bulundu", other: "{count} dosya bulundu" },
    none: "Bu türlerde dosya bulunamadı.",
    canceled: "Erken durduruldu: bunlar o ana kadar bulunan dosyalar.",
    pageLimit: "Her sayfa okunmadan sayfa sınırına ulaşıldı.",
    blocked: { one: "robots.txt izin vermediği için {count} sayfa ya da dosya atlandı.", other: "robots.txt izin vermediği için {count} sayfa ya da dosya atlandı." },
    failed: { one: "{count} sayfa okunamadı.", other: "{count} sayfa okunamadı." },
    filter: "Dosyaları süz",
    selectAll: "Gösterilen tüm dosyaları seç",
    name: "Ad",
    size: "Boyut",
    foundOn: "Bulunduğu sayfa",
    selected: { one: "{count} seçili, {size}", other: "{count} seçili, {size}" },
    unknownSizes: { one: "{count} tanesinin boyutu bilinmiyor", other: "{count} tanesinin boyutu bilinmiyor" },
    add: { one: "{count} dosya ekle", other: "{count} dosya ekle" },
    added: { one: "{count} dosya eklendi.", other: "{count} dosya eklendi." },
    alreadyListed: { one: "{count} tanesi zaten listedeydi.", other: "{count} tanesi zaten listedeydi." },
  },
  errors: {
    invalid_url: "Bir http ya da https adresi girin.",
    no_types: "En az bir dosya türü seçin.",
    robots_blocked: "Bu sitenin robots.txt dosyası tarayıcıların bu sayfayı okumamasını istiyor. Yine de okumak için robots.txt kurallarına uy seçeneğini kapatın.",
    start_page: "Başlangıç sayfası okunamadı: {error}",
    not_html: "Başlangıç adresi bir web sayfası değil. Onu bir indirme olarak ekleyin.",
  },
};

export const feeds: Messages["feeds"] = {
  open: "Akışlar",
  eyebrow: "Akışlar",
  title: "RSS ve Atom akışları",
  intro: "Downpipe her akışı kendi aralığında denetler ve yeni öğelerin dosyalarını seçtiğiniz kuyruğa ekler. Bir öğe yalnızca bir kez eklenir; indirmesini kaldırsanız bile.",
  empty: "Henüz akış yok. Bir podcast, bir sürüm akışı ya da başka bir RSS veya Atom adresi ekleyin.",
  add: "Akış ekle",
  checkNow: "Şimdi denetle",
  edit: "Düzenle",
  off: "Kapalı",
  checked: "Denetlendi: {date}",
  notChecked: "Henüz denetlenmedi",
  error: "Son denetim başarısız oldu: {error}",
  addedTotal: { one: "Toplam {count} indirme eklendi", other: "Toplam {count} indirme eklendi" },
  checkResult: {
    none: "{name} içinde yeni öğe yok.",
    added: { one: "{name} akışından {count} indirme eklendi.", other: "{name} akışından {count} indirme eklendi." },
  },
  confirmRemove: "{name} kaldırılsın mı? Eklediği indirmeler listede kalır.",
  form: {
    newTitle: "Yeni akış",
    editTitle: "Akışı düzenle",
    url: "Akış adresi",
    types: "Dosya türleri",
    typesHint: "Boş: öğelerin sunduğu her dosya. Örneğin mp3, m4a.",
    pattern: "Başlık süzgeci",
    patternHint: "Başlığın içermesi gereken bir düzenli ifade, büyük-küçük harf fark etmez; örneğin bölüm|bonus. Boş: her başlık.",
    interval: "Denetim sıklığı",
    queue: "Kuyruk",
    enabled: "Bu akışı denetle",
    enabledHint: "Kapalı: akış listede kalır ama denetlenmez.",
    existing: "Akışta şu an olanları da indir",
    existingHint: "Kapalı: yalnızca bundan sonra yayımlanan öğeler indirilir.",
    save: "Kaydet",
    subscribe: "Akışı ekle",
    subscribed: { one: "Akış eklendi. {count} indirme eklendi.", other: "Akış eklendi. {count} indirme eklendi." },
    subscribedNone: "Akış eklendi. Bundan sonra yeni öğeler indirilecek.",
  },
  interval: {
    minutes: { one: "{count} dakika", other: "{count} dakika" },
    hours: { one: "{count} saat", other: "{count} saat" },
    days: { one: "{count} gün", other: "{count} gün" },
  },
  errors: {
    duplicate_feed: "Bu akış zaten listede.",
    not_a_feed: "Bu adres bir web sayfası, RSS ya da Atom akışı değil.",
    invalid_url: "Bir http ya da https adresi girin.",
  },
};

export const hosts: Messages["hosts"] = {
  privateFile: "Dosya herkese açık paylaşılmamış. Sahibinden bağlantıya sahip herkesle paylaşmasını isteyin ya da oturum açıkken tarayıcınızda indirin.",
};
