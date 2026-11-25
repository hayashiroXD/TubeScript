# TubeScript

**m.youtube.com** üzerine kurulu, içine gerçek bir **userscript motoru** gömülü Android uygulaması.
Tauri 2 + Rust ile yazıldı; Google Play Services gerektirmez (microG / GMS'siz ROM'larda çalışır).

> **English:** TubeScript is an Android shell around `m.youtube.com` with a built-in userscript
> engine (GM_* API), cosmetic filters and a native-feeling UI. Built with Tauri 2 + Rust.
> No Google Play Services required. Licensed under Apache-2.0.

---

## İndir

Son imzalı APK: **[Releases →](https://github.com/hayashiroXD/TubeScript/releases/latest)**

- Paket: `app.tubescript.android`
- Sürüm: 1.1.0
- Android 8.0+ (minSdk 26, targetSdk 37)
- Mimari: `arm64-v8a`
- İmza: APK Signature Scheme v2/v3

```bash
adb install -r TubeScript.apk
```

APK'yı telefona kopyalayıp "bilinmeyen kaynaklara izin ver" ile de kurabilirsin.

---

## Ne yapar?

- **Gerçek YouTube arayüzü.** Uygulama `m.youtube.com`'u Chrome mobil user-agent ile açar;
  YouTube'un kendi mobil arayüzü, Shorts düzeni, oynatıcısı aynen çalışır. Adres çubuğu yoktur.
- **Uygulama kabuğu.** Ana ekran, YouTube'da arama, Script'ler ve Ayarlar sekmeleri; alt menü,
  koyu tema, çentik/status bar uyumu.
- **Userscript motoru.** Script yaz, düzenle, aç/kapat; `.user.js` URL'sinden kur; `@name`,
  `@match`, `@run-at` başlıklarını otomatik oku.
- **Kozmetik filtreler.** Reklam/promo temizliği, Shorts'u gizle, yorumları gizle, ana akışı sadeleştir.
- **Native hissiyat.** Metin seçimi, uzun basma menüsü, tap parlaması ve çift dokunma zoom'u kapalı
  (varsayılan açık gelen `Native Hissiyat` userscript'i).
- **Sayfa üstü kontroller.** YouTube üzerinde `‹` geri, `{}` script'ler, `⚙` ayarlar, `⟳` yenile.
- **Donanım geri tuşu.** WebView geçmişinde geri gider; kök sayfada uygulamadan çıkar.
- **GMS'siz.** Hiçbir Google Play Services çağrısı yok; tek izin `android.permission.INTERNET`.

---

## Userscript desteği

Script'ler `m.youtube.com` ve `www.youtube.com` sayfalarına enjekte edilir. Sayfa bağlamında
(`window`) çalışırlar; `GM` nesnesi ve klasik `GM_*` fonksiyonları birlikte sunulur:

| API | Açıklama |
| --- | --- |
| `GM.addStyle(css)` | Sayfaya `<style>` ekler |
| `GM.getValue(k, def)` / `GM_setValue(k, v)` | Kalıcı anahtar/değer (localStorage) |
| `GM.deleteValue(k)` / `GM_listValues()` | Silme / listeleme |
| `GM.xmlhttpRequest({url, method, headers, data, onload, onerror})` | `fetch` tabanlı istek |
| `GM_download(url, name)` | Dosya indirme |
| `GM_notification(title, opts)` | Bildirim (konsol) |
| `GM_openInTab(url)` / `GM_setClipboard(text)` | Sekme / pano |
| `GM_info` | Script adı ve sürüm bilgisi |
| `GM.log(...)` / `GM.warn(...)` | Konsol |

Desteklenen başlıklar: `@name`, `@match`, `@include`, `@run-at`.
Eşleşme kalıpları glob olarak yorumlanır (`https://m.youtube.com/*`).

### Hazır script'ler

- Reklam & Promo Temizleyici
- Native Hissiyat (seçim/uzun basma kapalı)
- Shorts Rafını Gizle
- Otomatik Oynatmayı Kapat
- Varsayılan Oynatma Hızı: 1.5x

---

## Kaynaktan derleme

Gerekenler:

- Node.js 20+
- JDK 17
- Android SDK: platform 36/37, build-tools 36, platform-tools
- NDK 27 (`27.1.12297006`)
- Rust + `rustup target add aarch64-linux-android`

```bash
git clone https://github.com/hayashiroXD/TubeScript.git
cd TubeScript
npm install
./build-apk.sh            # imzalı release APK
./build-apk.sh debug      # debug APK
```

Betik sonunda imzalı APK hem `src-tauri/gen/android/app/build/outputs/apk/` altında
hem de proje klasörünün bir üstünde (`../TubeScript.apk`) durur.

### İmzalama

`build-apk.sh`, `keystore/tubescript.jks` dosyasını kullanır. Bu dosya **repoda yoktur**.
Kendi anahtarını üret:

```bash
mkdir -p keystore
keytool -genkeypair -v -keystore keystore/tubescript.jks \
  -alias tubescript -keyalg RSA -keysize 2048 -validity 10000 \
  -storepass <parola> -keypass <parola> \
  -dname "CN=TubeScript, O=TubeScript, C=TR"
```

Ardından `src-tauri/gen/android/keystore.properties` dosyasını kendi parolanla oluştur
(betik bunu otomatik yazar; parolayı `build-apk.sh` içinden değiştir).

---

## Mimari

```
ytlite/
├── src-tauri/
│   ├── src/lib.rs        # WebView kontrolü, userscript enjeksiyonu, komutlar
│   ├── tauri.conf.json   # pencere, user-agent, APK ayarları
│   ├── capabilities/     # Tauri izinleri
│   └── icons/            # uygulama ikonları
├── ui/                   # uygulama kabuğu (HTML/CSS/JS)
├── keystore/             # yerel imza anahtarı (git'e girmez)
└── build-apk.sh
```

Enjeksiyon: Rust tarafındaki bir döngü, WebView `youtube.com` üzerindeyken userscript
motorunu `eval` ile sayfaya bırakır. Motor `window.__tsApplied` ile bir kez çalışır;
ayar değişikliklerinden sonra YouTube'u yenilemek gerekir.

---

## Gizlilik ve uyarılar

- Uygulama hiçbir veriyi toplamaz, analitik/telemetri içermez.
- Google hesaplarıyla giriş, WebView üzerinden yapılır. Google, WebView girişlerini bazen
  engelleyebilir; uygulama Chrome mobil user-agent kullanarak bunu aşmaya çalışır.
- YouTube/Google ile bağlantılı değildir. "YouTube" ve "Google" ilgili şirketlerin tescilli
  markalarıdır.
- Userscript'ler sayfa bağlamında tam yetkiyle çalışır; yalnızca güvendiğin script'leri kur.
- Üçüncü taraf sitelerden APK indirirken imzayı doğrula; bu depodaki Releases dışındaki
  yapılar bu projeye ait değildir.

---

## Lisans

[Apache License 2.0](LICENSE) © 2026 Hayashiro
