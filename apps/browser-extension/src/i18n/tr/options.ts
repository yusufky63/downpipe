// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "Filtreler · Downpipe",
  headLabel: "Tarayıcı eklentisi",
  lede:
    "Tarayıcı indirmelerini yakala açıkken tarayıcının hangi indirmeleri Downpipe'a vereceğini, Bu sayfadaki medya açıkken eklentinin hangi medyayı listeleyeceğini seçin. Geri kalan her şey her zamanki gibi tarayıcıda kalır.",
  captureOff:
    "Yakalama kapalı. Bu filtreler, Downpipe araç çubuğu penceresinde Tarayıcı indirmelerini yakala seçeneğini açtığınızda uygulanır.",
  loadError: "Ayarlar yüklenemedi. Yeniden denemek için bu sayfayı yeniden yükleyin.",
  typesHeading: "01 — Dosya türleri",
  captureAll: "Tüm dosya türlerini yakala",
  captureAllDetail: "Tarayıcının başlattığı her indirme, türü ne olursa olsun Downpipe'a gider.",
  typesLabel: "Yalnızca bu dosya türleri",
  typesHelp:
    "Boşluk ya da virgülle ayrılmış uzantılar, ör. {example}. Dosya adı bunlardan biriyle biten bir indirme yakalanır; {tarGz} da çalışır.",
  restoreDefault: "Varsayılan listeyi geri yükle",
  sitesHeading: "02 — Hariç tutulan siteler",
  sitesLabel: "Bu sitelerde hiçbir zaman yakalama ve medya listeleme",
  sitesHelp:
    "Her satıra bir site. {wildcard} tüm alt alan adlarını da kapsar. Bu sitelerden birinden gelen ya da sayfalarından birinde başlayan bir indirme tarayıcıda kalır ve bu sitelerin oynattığı medya listelenmez. Bağlantı noktaları ve yollar yok sayılır.",
  sizeHeading: "03 — En küçük boyut",
  sizeLabel: "Daha küçük dosyaları tarayıcıya bırak",
  sizeHelp: "0 bunu kapatır. Sunucu dosyanın boyutunu bildirmediğinde dosya Downpipe'a gider.",
  mediaHeading: "04 — Sayfalardaki medya",
  mediaHelp:
    "Araç çubuğu penceresinde Bu sayfadaki medya açıkken kullanılır. HLS ve DASH akışları her zaman listelenir; yukarıdaki hariç tutulan siteler burada da geçerlidir.",
  overlay: "Videoların üzerinde İndir düğmesi göster",
  overlayDetail: "Oynatılan bir videoda, birkaç saniye boyunca ve işaretçi üzerindeyken.",
  mediaTypesLabel: "Bu türlerdeki medya dosyalarını listele",
  mediaTypesHelp:
    "Boşluk ya da virgülle ayrılmış uzantılar. Uzantısı olmayan bir dosya, sunucunun gönderdiği türe göre değerlendirilir. Boş bir liste yalnızca akışları listeler.",
  mediaSizeLabel: "Şundan küçük medya dosyalarını dışarıda bırak",
  mediaSizeHelp:
    "Bildirim seslerini ve kısa önizlemeleri dışarıda tutar. 0 her boyutu listeler; boyutunu sunucunun bildirmediği dosyalar her zaman listelenir.",
  bypassHeading: "05 — Tarayıcıda indir",
  bypassLabel: "Şu tuşla tıklanan bağlantıyı tarayıcı indirsin",
  bypassAlt: "Alt (Mac'te Option)",
  bypassShift: "Shift",
  bypassOff: "Kapalı",
  bypassHelp: "Yakalama açıkken bu tuş basılıyken bir bağlantıya tıklarsanız o indirme tarayıcıda kalır. Diğer tıklamalar ve tarayıcının kendi kısayolları değişmez.",
  bypassAccess: "Tıklamayı fark etmek için ziyaret ettiğiniz sitelere erişim gerekir. Bir bağlantıya bu şekilde tıklayana kadar hiçbir şey okunmaz ve gönderilmez.",
  bypassAllow: "Erişime izin ver",
  save: "Kaydet",
  moreErrors: { one: "Ve {count} tane daha.", other: "Ve {count} tane daha." },
  unsaved: "Kaydedilmemiş değişiklikler",
  notSavedFix: "Kaydedilmedi. Kırmızıyla işaretlenen alanları düzeltin.",
  saving: "Kaydediliyor…",
  notSavedNoAnswer: "Kaydedilmedi. Eklenti yanıt vermedi; yeniden deneyin.",
  saved: "Kaydedildi",
  errors: {
    notExtension: "“{value}” bir dosya uzantısı değil.",
    tooManyTypes: { one: "En fazla {count} dosya türü listeleyin.", other: "En fazla {count} dosya türü listeleyin." },
    notSite: "Satır {line}: “{value}” bir site değil. example.com ya da *.example.com gibi bir ana bilgisayar adı kullanın.",
    tooManySites: { one: "En fazla {count} site listeleyin.", other: "En fazla {count} site listeleyin." },
    sizeNotNumber: "2 ya da 1.5 gibi bir sayı girin; kapatmak için 0 girin.",
    sizeTooBig: "1 TiB'tan küçük bir boyut girin.",
    noTypes: "En az bir dosya türü ekleyin ya da Tüm dosya türlerini yakala seçeneğini açın.",
  },
};
