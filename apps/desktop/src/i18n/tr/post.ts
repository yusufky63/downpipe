// After a download completes: the virus scan, extracting archives and a command.
import type { Messages } from "../core";

export const post: Messages["post"] = {
  steps: {
    scanning: "Taranıyor",
    extracting: "Çıkarılıyor",
    running_command: "Komut çalışıyor",
  },
  threat: "Tehdit",
  summary: {
    threat: "Tehdit bulundu: {threat}",
    notExtracted: "Çıkarılmadı",
    commandFailed: "İndirme sonrası komut başarısız oldu",
  },
  section: "İndirme bitince",
  scan: {
    label: "Virüs taraması",
    clean: "{scanner} bir şey bulmadı.",
    threat:
      "{scanner}, {threat} buldu. Hiçbir şey çıkarılmadı veya çalıştırılmadı ve dosya olduğu gibi bırakıldı: silip silmemek size kalmış.",
    unnamed: "bir tehdit",
    theScanner: "Tarayıcı",
    missing: "Taranmadı: kurulu bir tarayıcı yok. Ayarlar, İndirme bitince bölümünden birini seçin.",
    missingNamed: "Taranmadı: {scanner} bulunamadı.",
    failed: "{scanner} dosyayı tarayamadı: {detail}",
  },
  extract: {
    label: "Çıkarıldı",
    done: { one: "{count} dosya yeni bir klasöre.", other: "{count} dosya yeni bir klasöre." },
    skipped: {
      one: "{count} öğe dışarıda bırakıldı: güvensiz bir ad, bir bağlantı ya da bir kopya.",
      other: "{count} öğe dışarıda bırakıldı: güvensiz adlar, bağlantılar ya da kopyalar.",
    },
    deleted: "Arşiv ardından silindi.",
    openFolder: "Klasörü aç",
    failed: "Çıkarılmadı: {reason}.",
    errors: {
      encrypted: "arşiv parolayla korunuyor",
      too_large: "kendi boyutunun çok ötesine açılırdı (olası bir zip bombası)",
      too_many_entries: "çok fazla öğesi var",
      corrupt: "arşiv bozuk ya da desteklenmeyen bir yöntem kullanıyor",
      io: "dosyalar yazılamadı",
    },
  },
  command: {
    label: "Komut",
    ok: "{program} tamamlandı.",
    failed: "{program} {code} çıkış koduyla sonlandı.",
    ended: "{program} bitmeden sonlandırıldı.",
    timedOut: "{program} süre sınırını aştı ve durduruldu.",
    notStarted: "{program} başlatılamadı: {detail}",
    invalid: "Komut çalıştırılamıyor: {detail}",
    theCommand: "Komut",
    output: "Çıktı",
  },
  fields: {
    heading: "İndirme bitince",
    command: "Bitince çalıştır",
    inherited: "Varsayılan: {command}",
    nothing: "Hiçbir şey",
    hint: "Bir program ve argümanları, örneğin notepad {path}. {path}, {name}, {folder} ve {url} doldurulur, her biri tek bir argüman olarak ve kabuk kullanılmadan. İndirme başarısız olursa çalışmaz.",
    extract: "Arşivi çıkar",
    extractHint: "Tamamlandığında, yanındaki yeni bir klasöre.",
    save: "Kaydet",
    saved: "Kaydedildi. İndirme tamamlandığında uygulanır.",
  },
  settings: {
    tab: "İndirme bitince",
    command: "Her indirmeden sonra bir komut çalıştır",
    commandHint:
      "Bir program ve argümanları, örneğin notepad {path}. {path}, {name}, {folder} ve {url} doldurulur, her biri tek bir argüman olarak; hiçbir şey kabuktan geçmez, bu yüzden dosya adları ve bağlantılar komut ekleyemez. Kuyruklar ve tek tek indirmeler kendi komutlarını belirleyebilir. İndirme başarısız olursa çalışmaz.",
    timeout: "Süre sınırı",
    timeoutHint: "Bir komutun ya da taramanın, başlattığı her şeyle birlikte durdurulmadan önce çalışabileceği saniye. 5 ile 86400 arası.",
    seconds: "sn",
    scan: "İndirmeleri virüslere karşı tara",
    scanHintWindows:
      "Microsoft Defender ile, her indirme tamamlandıktan sonra. Bir tehdit indirmeyi işaretler ve bildirim gösterir; hiçbir şey silinmez ya da karantinaya alınmaz, hiçbir şey çıkarılmaz ya da çalıştırılmaz.",
    scanHintOther:
      "Aşağıdaki tarayıcıyla, her indirme tamamlandıktan sonra. Bir tehdit indirmeyi işaretler ve bildirim gösterir; hiçbir şey silinmez, çıkarılmaz ya da çalıştırılmaz.",
    scanner: "Tarayıcı",
    scannerPlaceholderWindows: "Microsoft Defender",
    scannerHint:
      "Boş bırakılırsa Windows'ta Microsoft Defender kullanılır. Başka bir tarayıcı dosyayı {path} olarak ya da son argüman olarak alır; ClamAV'nin clamscan'inde olduğu gibi 0 çıkış kodu temiz, 1 tehdit demektir.",
    archives: "Arşivler",
    archivesHint:
      "Zip, tar, tar.gz, tar.xz ve 7z arşivleri, kategorileri (Kategoriler) ya da indirmenin kendisi öyle diyorsa yanlarındaki yeni bir klasöre çıkarılır. O klasörün dışına düşecek öğeler, bağlantılar ve kendi boyutunun çok ötesine açılan arşivler reddedilir. RAR desteklenmez; 7-Zip onu bir komut olarak çıkarabilir.",
    deleteArchive: "Arşivi çıkardıktan sonra sil",
    deleteArchiveHint: "Yalnızca her öğe çıkarıldığında ve komut çalıştıktan sonra.",
    queueCommand: "{name} indirmelerinden sonra komut",
    queueCommandUnnamed: "Kuyruğun indirmelerinden sonra komut",
    queueCommandPlaceholder: "Bitince komut; boş: İndirme bitince'deki",
    categoryExtract: "Arşivleri çıkar",
    categoryExtractTitle: "Bu kategorinin zip, tar ve 7z arşivlerini indikten sonra çıkar",
  },
};
