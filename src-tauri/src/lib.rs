use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, State, Url};

// ---------------------------------------------------------------------------
// Veri modeli
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub matches: Vec<String>,
    pub run_at: String,
    pub code: String,
    pub updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub adblock: bool,
    pub hide_shorts: bool,
    pub hide_home_feed: bool,
    pub hide_comments: bool,
    pub inject_fab: bool,
    #[serde(default)]
    pub local_origin: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            adblock: true,
            hide_shorts: false,
            hide_home_feed: false,
            hide_comments: false,
            inject_fab: true,
            local_origin: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Store {
    pub scripts: Vec<Script>,
    pub settings: Settings,
}

type Shared = Arc<Mutex<Store>>;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn default_store() -> Store {
    let seed_ad = Script {
        id: "seed-adclean".into(),
        name: "Reklam & Promo Temizleyici".into(),
        enabled: true,
        matches: vec![
            "https://m.youtube.com/*".into(),
            "https://www.youtube.com/*".into(),
        ],
        run_at: "document-idle".into(),
        updated: now_ms(),
        code: r#"// YouTube mobil web reklam / promosyon temizleyici
(function () {
  const SEL = [
    'ytm-promoted-sparkles-web-renderer',
    'ytm-companion-ad-renderer',
    'ytm-promo-renderer',
    'ytm-display-ad-renderer',
    'ytm-in-feed-ad-layout-renderer',
    'ytm-banner-promo-renderer',
    'ytm-statement-banner-renderer',
    'ytm-app-promo-renderer',
    '[data-ad-slot]',
    '.ytp-ad-module',
    '.ytp-ad-overlay-slot',
    '.ytp-ad-image-overlay'
  ].join(',');

  const css = SEL + '{display:none !important;}';
  const style = document.createElement('style');
  style.id = 'ts-seed-adclean';
  style.textContent = css;
  (document.head || document.documentElement).appendChild(style);

  const sweep = () => document.querySelectorAll(SEL).forEach((el) => el.remove());
  sweep();
  setInterval(sweep, 1500);
})();"#
            .into(),
    };

    let seed_shorts = Script {
        id: "seed-shorts".into(),
        name: "Shorts Rafını Gizle".into(),
        enabled: false,
        matches: vec!["https://m.youtube.com/*".into()],
        run_at: "document-idle".into(),
        updated: now_ms(),
        code: r#"// Ana sayfadaki Shorts raflarını ve sekmesini gizler
GM.addStyle([
  'ytm-reel-shelf-renderer',
  'ytm-shorts-lockup-view-model',
  'ytm-shorts-lockup-view-model-v2',
  'ytm-pivot-bar-item-renderer:has(a[href*="shorts"])'
].join(',') + '{display:none !important;}');

GM.log('Shorts gizlendi');"#
            .into(),
    };

    let seed_native = Script {
        id: "seed-native".into(),
        name: "Native Hissiyat (seçim/uzun basma kapalı)".into(),
        enabled: true,
        matches: vec![
            "https://m.youtube.com/*".into(),
            "https://www.youtube.com/*".into(),
        ],
        run_at: "document-start".into(),
        updated: now_ms(),
        code: r#"// Web uygulaması hissini kıran etkileşimleri kapatır:
// metin seçimi, uzun basma menüsü, sürükleme, mavi tap parlaması, çift dokunma zoom'u.
(function () {
  GM.addStyle([
    'html, body, ytm-app, #app {',
    '  -webkit-user-select: none !important;',
    '  user-select: none !important;',
    '  -webkit-touch-callout: none !important;',
    '  -webkit-tap-highlight-color: transparent !important;',
    '  overscroll-behavior: none !important;',
    '  touch-action: manipulation;',
    '}',
    'input, textarea, [contenteditable="true"] {',
    '  -webkit-user-select: text !important;',
    '  user-select: text !important;',
    '  -webkit-touch-callout: default !important;',
    '}',
    'img, a { -webkit-user-drag: none !important; user-drag: none !important; }',
    '::-webkit-scrollbar { width: 0 !important; height: 0 !important; }'
  ].join('\n'));

  function isEditable(el) {
    return !!(el && el.closest && el.closest('input, textarea, [contenteditable="true"]'));
  }

  ['contextmenu', 'dragstart', 'selectstart'].forEach(function (type) {
    document.addEventListener(type, function (e) {
      if (isEditable(e.target)) return;
      e.preventDefault();
    }, true);
  });

  GM.log('Native hissiyat aktif');
})();"#
            .into(),
    };

    Store {
        scripts: vec![seed_ad, seed_native, seed_shorts],
        settings: Settings::default(),
    }
}

fn store_path(app: &AppHandle) -> PathBuf {
    let dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::env::temp_dir());
    let _ = fs::create_dir_all(&dir);
    dir.join("tubescript.json")
}

fn persist(app: &AppHandle, store: &Store) {
    if let Ok(txt) = serde_json::to_string_pretty(store) {
        let _ = fs::write(store_path(app), txt);
    }
}

fn load_store(app: &AppHandle) -> Store {
    fs::read_to_string(store_path(app))
        .ok()
        .and_then(|txt| serde_json::from_str::<Store>(&txt).ok())
        .unwrap_or_else(default_store)
}

// ---------------------------------------------------------------------------
// Enjeksiyon motoru
// ---------------------------------------------------------------------------

const INJECT_TEMPLATE: &str = r#"(function () {
  if (!/(^|\.)youtube\.com$/.test(location.hostname)) return;
  if (window.__tsApplied) return;
  window.__tsApplied = true;

  var D = __DATA__;
  var MONKEY_KEY = 'ts_gm_';
  var state = { style: null, fab: null };

  // ---------- kozmetik filtreler ----------
  function buildCss() {
    var css = '';
    var s = D.settings;
    if (s.adblock) {
      css += [
        'ytm-promoted-sparkles-web-renderer',
        'ytm-companion-ad-renderer',
        'ytm-promo-renderer',
        'ytm-display-ad-renderer',
        'ytm-in-feed-ad-layout-renderer',
        'ytm-banner-promo-renderer',
        'ytm-statement-banner-renderer',
        'ytm-app-promo-renderer',
        '[data-ad-slot]',
        '.ytp-ad-module',
        '.ytp-ad-overlay-slot',
        '.ytp-ad-image-overlay',
        'ytm-mealbar-promo-renderer'
      ].map(function (x) { return x + '{display:none!important}'; }).join('\n');
    }
    if (s.hideShorts) {
      css += '\n' + [
        'ytm-reel-shelf-renderer',
        'ytm-shorts-lockup-view-model',
        'ytm-shorts-lockup-view-model-v2',
        'ytm-pivot-bar-item-renderer:has(a[href*="shorts"])'
      ].map(function (x) { return x + '{display:none!important}'; }).join('\n');
    }
    if (s.hideHomeFeed) {
      css += '\n' + [
        'ytm-chip-cloud-renderer',
        'ytm-rich-section-renderer',
        'ytm-promoted-sparkles-web-renderer'
      ].map(function (x) { return x + '{display:none!important}'; }).join('\n');
    }
    if (s.hideComments) {
      css += '\n' + [
        'ytm-comments-entry-point-header-renderer',
        'ytm-comment-section-renderer',
        'ytm-comments-entry-point-teaser-renderer'
      ].map(function (x) { return x + '{display:none!important}'; }).join('\n');
    }
    // YouTube'un "uygulamada aç" çağrıları ve boş alanlar
    css += '\n.ytp-pause-overlay{display:none!important}';
    return css;
  }

  function applyStyle() {
    var css = buildCss();
    if (!css) return;
    if (state.style && state.style.isConnected) {
      if (state.style.textContent !== css) state.style.textContent = css;
      return;
    }
    var el = document.createElement('style');
    el.id = 'ts-style';
    el.textContent = css;
    (document.head || document.documentElement).appendChild(el);
    state.style = el;
  }

  // ---------- userscript API (GM_* shim) ----------
  function readVal(k, def) {
    try {
      var raw = localStorage.getItem(MONKEY_KEY + k);
      return raw === null ? def : JSON.parse(raw);
    } catch (e) { return def; }
  }
  function writeVal(k, v) {
    try { localStorage.setItem(MONKEY_KEY + k, JSON.stringify(v)); } catch (e) {}
  }
  function delVal(k) {
    try { localStorage.removeItem(MONKEY_KEY + k); } catch (e) {}
  }

  function makeGM(meta) {
    var ns = meta.id + '::';
    var GM = {
      info: {
        script: { name: meta.name, version: '1.0', namespace: 'tubescript' },
        scriptHandler: 'TubeScript',
        version: '1.0.0'
      },
      addStyle: function (css) {
        var el = document.createElement('style');
        el.setAttribute('data-ts-script', meta.id);
        el.textContent = css;
        (document.head || document.documentElement).appendChild(el);
        return el;
      },
      getValue: function (k, d) { return readVal(ns + k, d); },
      setValue: function (k, v) { writeVal(ns + k, v); return v; },
      deleteValue: function (k) { delVal(ns + k); },
      listValues: function () {
        var out = [];
        for (var i = 0; i < localStorage.length; i++) {
          var key = localStorage.key(i) || '';
          if (key.indexOf(MONKEY_KEY + ns) === 0) out.push(key.slice((MONKEY_KEY + ns).length));
        }
        return out;
      },
      log: function () {
        var args = ['[userscript:' + meta.name + ']'].concat([].slice.call(arguments));
        console.log.apply(console, args);
      },
      warn: function () { console.warn.apply(console, ['[' + meta.name + ']'].concat([].slice.call(arguments))); },
      notification: function (title, opts) {
        console.log('[userscript:' + meta.name + '] ' + title, opts || '');
      },
      openInTab: function (url) { window.open(url, '_blank'); },
      setClipboard: function (text) {
        try { if (navigator.clipboard) navigator.clipboard.writeText(text); } catch (e) {}
      },
      download: function (url, name) {
        var a = document.createElement('a');
        a.href = url; a.download = name || ''; a.click();
      },
      registerMenuCommand: function () {},
      xmlhttpRequest: function (details) {
        try {
          fetch(details.url, {
            method: details.method || 'GET',
            headers: details.headers || {},
            body: details.data
          }).then(function (r) {
            return r.text().then(function (t) {
              if (details.onload) details.onload({ status: r.status, responseText: t, finalUrl: r.url });
            });
          }).catch(function (e) {
            if (details.onerror) details.onerror(e);
          });
        } catch (e) {
          if (details.onerror) details.onerror(e);
        }
      }
    };
    return GM;
  }

  function globToRegex(pattern) {
    var escaped = pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    escaped = escaped.replace(/\\\*/g, '.*');
    return new RegExp('^' + escaped + '$');
  }

  function runScripts() {
    var list = D.scripts || [];
    for (var i = 0; i < list.length; i++) {
      var s = list[i];
      try {
        var ok = !s.matches || s.matches.length === 0;
        if (!ok) {
          for (var j = 0; j < s.matches.length; j++) {
            if (globToRegex(s.matches[j]).test(location.href)) { ok = true; break; }
          }
        }
        if (!ok) continue;
        var GM = makeGM(s);
        var runner = new Function(
          'window', 'document', 'unsafeWindow', 'GM', 'GM_info',
          'GM_addStyle', 'GM_getValue', 'GM_setValue', 'GM_deleteValue', 'GM_listValues',
          'GM_xmlhttpRequest', 'GM_log', 'GM_notification', 'GM_openInTab', 'GM_setClipboard',
          'GM_download', 'GM_registerMenuCommand',
          'console', 'location', 'navigator', 'fetch', 'setTimeout', 'setInterval',
          '"use strict";\n' + s.code + '\n//# sourceURL=tubescript://' + s.id
        );
        runner.call(
          window, window, document, window, GM, GM.info,
          GM.addStyle, GM.getValue, GM.setValue, GM.deleteValue, GM.listValues,
          GM.xmlhttpRequest, GM.log, GM.notification, GM.openInTab, GM.setClipboard,
          GM.download, GM.registerMenuCommand,
          console, location, navigator, window.fetch ? window.fetch.bind(window) : undefined,
          window.setTimeout.bind(window), window.setInterval.bind(window)
        );
        console.log('[TubeScript] çalıştı: ' + s.name);
      } catch (err) {
        console.error('[TubeScript] script hatası: ' + s.name, err);
      }
    }
  }

  // ---------- yüzen düğmeler ----------
  function makeBtn(glyph, title) {
    var b = document.createElement('a');
    b.textContent = glyph;
    b.title = title;
    b.setAttribute('role', 'button');
    b.setAttribute('style', [
      'width:48px', 'height:48px', 'border-radius:24px',
      'display:flex', 'align-items:center', 'justify-content:center',
      'background:rgba(28,28,28,.94)', 'color:#fff', 'font-size:21px',
      'text-decoration:none', 'box-shadow:0 6px 18px rgba(0,0,0,.5)',
      'border:1px solid rgba(255,255,255,.14)', 'backdrop-filter:blur(6px)'
    ].join(';'));
    return b;
  }

  function mountFab() {
    if (!D.settings.injectFab) return;
    if (state.fab && state.fab.isConnected) return;
    var wrap = document.createElement('div');
    wrap.id = 'ts-fab';
    wrap.setAttribute('style', [
      'position:fixed', 'right:10px', 'bottom:88px', 'z-index:2147483646',
      'display:flex', 'flex-direction:column', 'gap:10px',
      'font-family:Roboto,system-ui,sans-serif'
    ].join(';'));

    var back = makeBtn('‹', 'Geri');
    back.href = '#';
    back.addEventListener('click', function (e) { e.preventDefault(); history.back(); });
    wrap.appendChild(back);

    var code = makeBtn('{}', 'Userscriptler');
    code.href = D.origin ? D.origin + '/?view=scripts' : '#';
    wrap.appendChild(code);

    var gear = makeBtn('⚙', 'TubeScript ayarları');
    gear.href = D.origin ? D.origin + '/?view=settings' : '#';
    wrap.appendChild(gear);

    var reload = makeBtn('⟳', 'Sayfayı yenile');
    reload.href = '#';
    reload.addEventListener('click', function (e) { e.preventDefault(); location.reload(); });
    wrap.appendChild(reload);

    (document.body || document.documentElement).appendChild(wrap);
    state.fab = wrap;
  }

  // ---------- ilk çalıştırma + SPA senkronu ----------
  applyStyle();
  mountFab();
  runScripts();

  var queued = false;
  function sync() {
    if (queued) return;
    queued = true;
    setTimeout(function () {
      queued = false;
      applyStyle();
      mountFab();
    }, 220);
  }

  try {
    new MutationObserver(sync).observe(document.documentElement, { childList: true, subtree: true });
  } catch (e) {}
  window.addEventListener('yt-navigate-finish', sync, true);
  window.addEventListener('yt-page-data-updated', sync, true);
})();"#;

fn injection_script(store: &Store) -> String {
    let scripts: Vec<serde_json::Value> = store
        .scripts
        .iter()
        .filter(|s| s.enabled)
        .map(|s| {
            json!({
                "id": s.id,
                "name": s.name,
                "matches": s.matches,
                "runAt": s.run_at,
                "code": s.code,
            })
        })
        .collect();

    let data = json!({
        "scripts": scripts,
        "settings": store.settings,
        "origin": store.settings.local_origin,
        "ts": now_ms(),
    });

    let mut encoded = serde_json::to_string(&data).unwrap_or_else(|_| "{}".into());
    // JS string literal icinde satir ayiricilarini guvenli hale getir
    encoded = encoded.replace('\u{2028}', "\\u2028").replace('\u{2029}', "\\u2029");

    INJECT_TEMPLATE.replace("__DATA__", &encoded)
}

// ---------------------------------------------------------------------------
// WebView yardimcilari
// ---------------------------------------------------------------------------

fn main_webview(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    app.get_webview_window("main")
        .or_else(|| app.webview_windows().into_values().next())
}

fn navigate(app: &AppHandle, target: &str) -> Result<(), String> {
    let url = Url::parse(target).map_err(|e| format!("geçersiz URL: {e}"))?;
    let wv = main_webview(app).ok_or("webview bulunamadı")?;
    wv.navigate(url).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Komutlar
// ---------------------------------------------------------------------------

#[tauri::command]
fn get_state(state: State<'_, Shared>) -> Store {
    state.lock().unwrap().clone()
}

#[tauri::command]
fn save_script(app: AppHandle, state: State<'_, Shared>, mut script: Script) -> Result<Store, String> {
    if script.name.trim().is_empty() {
        return Err("Script adı boş olamaz".into());
    }
    if script.matches.is_empty() {
        script.matches = vec!["https://m.youtube.com/*".into()];
    }
    script.updated = now_ms();
    let mut guard = state.lock().unwrap();
    if let Some(existing) = guard.scripts.iter_mut().find(|s| s.id == script.id) {
        *existing = script;
    } else {
        if script.id.trim().is_empty() {
            script.id = format!("us-{}", now_ms());
        }
        guard.scripts.push(script);
    }
    persist(&app, &guard);
    Ok(guard.clone())
}

#[tauri::command]
fn delete_script(app: AppHandle, state: State<'_, Shared>, id: String) -> Result<Store, String> {
    let mut guard = state.lock().unwrap();
    guard.scripts.retain(|s| s.id != id);
    persist(&app, &guard);
    Ok(guard.clone())
}

#[tauri::command]
fn save_settings(app: AppHandle, state: State<'_, Shared>, settings: Settings) -> Result<Store, String> {
    let mut guard = state.lock().unwrap();
    let origin = guard.settings.local_origin.clone();
    guard.settings = settings;
    guard.settings.local_origin = origin;
    persist(&app, &guard);
    Ok(guard.clone())
}

#[tauri::command]
fn register_origin(state: State<'_, Shared>, origin: String) {
    let mut guard = state.lock().unwrap();
    if guard.settings.local_origin != origin {
        guard.settings.local_origin = origin;
    }
}

#[tauri::command]
fn add_preset(app: AppHandle, state: State<'_, Shared>, preset: String) -> Result<Store, String> {
    let (name, code) = match preset.as_str() {
        "shorts" => (
            "Shorts Rafını Gizle",
            r#"GM.addStyle([
  'ytm-reel-shelf-renderer',
  'ytm-shorts-lockup-view-model',
  'ytm-shorts-lockup-view-model-v2',
  'ytm-pivot-bar-item-renderer:has(a[href*="shorts"])'
].join(',') + '{display:none !important;}');
GM.log('Shorts gizlendi');"#,
        ),
        "autoplay" => (
            "Otomatik Oynatmayı Kapat",
            r#"var timer = setInterval(function () {
  var toggle = document.querySelector('ytm-autonav-toggle-button-renderer, .ytm-autonav-toggle-button');
  if (!toggle) return;
  var on = toggle.getAttribute('aria-checked') === 'true' || toggle.classList.contains('checked');
  if (on) { toggle.click(); GM.log('Otomatik oynatma kapatıldı'); }
  clearInterval(timer);
}, 1200);"#,
        ),
        "speed" => (
            "Varsayılan Oynatma Hızı: 1.5x",
            r#"setInterval(function () {
  var v = document.querySelector('video');
  if (v && v.playbackRate !== 1.5) v.playbackRate = 1.5;
}, 2000);"#,
        ),
        _ => (
            "Boş Script",
            r#"// Buraya kendi userscript kodunu yaz.
GM.log('Merhaba m.youtube.com');"#,
        ),
    };

    let mut guard = state.lock().unwrap();
    let id = format!("us-{}", now_ms());
    guard.scripts.push(Script {
        id,
        name: name.into(),
        enabled: true,
        matches: vec!["https://m.youtube.com/*".into()],
        run_at: "document-idle".into(),
        code: code.into(),
        updated: now_ms(),
    });
    persist(&app, &guard);
    Ok(guard.clone())
}

#[tauri::command]
fn reset_scripts(app: AppHandle, state: State<'_, Shared>) -> Result<Store, String> {
    let mut guard = state.lock().unwrap();
    let settings = guard.settings.clone();
    *guard = default_store();
    guard.settings = settings;
    persist(&app, &guard);
    Ok(guard.clone())
}

/// `.user.js` metninden @name / @match / @include / @run-at bilgilerini çıkarır.
fn parse_meta(code: &str) -> (String, Vec<String>, String) {
    let mut name: Option<String> = None;
    let mut matches: Vec<String> = Vec::new();
    let mut run_at = "document-idle".to_string();

    for line in code.lines().take(80) {
        let l = line.trim().trim_start_matches("//").trim();
        if !l.starts_with('@') {
            continue;
        }
        let mut parts = l.splitn(2, char::is_whitespace);
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("").trim();
        if value.is_empty() {
            continue;
        }
        match key {
            "@name" if name.is_none() => name = Some(value.to_string()),
            "@match" | "@include" => {
                if !value.contains("://") {
                    // @match https://m.youtube.com/* biçiminde olmalı; değilse atla
                    continue;
                }
                if !matches.iter().any(|m| m == value) {
                    matches.push(value.to_string());
                }
            }
            "@run-at" => {
                run_at = match value {
                    "document-start" => "document-start",
                    "document-end" => "document-end",
                    _ => "document-idle",
                }
                .to_string();
            }
            _ => {}
        }
    }

    if matches.is_empty() {
        matches.push("https://m.youtube.com/*".into());
    }

    (
        name.unwrap_or_else(|| "İndirilen userscript".into()),
        matches,
        run_at,
    )
}

#[tauri::command]
async fn install_from_url(
    app: AppHandle,
    state: State<'_, Shared>,
    url: String,
) -> Result<Store, String> {
    let target = url.trim();
    if !(target.starts_with("https://") || target.starts_with("http://")) {
        return Err("Geçerli bir http(s) adresi gir".into());
    }

    let resp = reqwest::get(target)
        .await
        .map_err(|e| format!("indirilemedi: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("sunucu {} döndü", resp.status()));
    }
    let code = resp
        .text()
        .await
        .map_err(|e| format!("içerik okunamadı: {e}"))?;

    if code.len() > 512 * 1024 {
        return Err("script çok büyük (512 KB üstü)".into());
    }
    if !code.contains("@") {
        return Err("bu adres bir userscript gibi görünmüyor".into());
    }

    let (name, matches, run_at) = parse_meta(&code);
    let mut guard = state.lock().unwrap();
    guard.scripts.push(Script {
        id: format!("us-{}", now_ms()),
        name,
        enabled: true,
        matches,
        run_at,
        code,
        updated: now_ms(),
    });
    persist(&app, &guard);
    Ok(guard.clone())
}

#[tauri::command]
fn open_youtube(app: AppHandle) -> Result<(), String> {
    navigate(&app, "https://m.youtube.com/?app=1")
}

#[tauri::command]
fn open_search(app: AppHandle, query: String) -> Result<(), String> {
    let q = query.trim();
    if q.is_empty() {
        return Err("arama metni boş".into());
    }
    let mut url = Url::parse("https://m.youtube.com/results").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("search_query", q);
    let wv = main_webview(&app).ok_or("webview bulunamadı")?;
    wv.navigate(url).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_home(app: AppHandle, state: State<'_, Shared>, view: Option<String>) -> Result<(), String> {
    let origin = state.lock().unwrap().settings.local_origin.clone();
    if origin.is_empty() {
        return Err("yerel kaynak henüz hazır değil".into());
    }
    let v = view.unwrap_or_else(|| "home".into());
    navigate(&app, &format!("{origin}/?view={v}"))
}

#[tauri::command]
fn reload_youtube(app: AppHandle) -> Result<(), String> {
    if let Some(wv) = main_webview(&app) {
        let _ = wv.eval("location.reload();");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Uygulama
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            let store = Arc::new(Mutex::new(load_store(&handle)));
            app.manage(store.clone());

            // YouTube sayfalarina periyodik olarak userscript motorunu enjekte et.
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_millis(1200));
                let injectable = {
                    let guard = store.lock().unwrap();
                    injection_script(&guard)
                };
                if let Some(wv) = main_webview(&handle) {
                    let on_youtube = wv
                        .url()
                        .map(|u| {
                            u.host_str()
                                .map(|h| h.ends_with("youtube.com"))
                                .unwrap_or(false)
                        })
                        .unwrap_or(false);
                    if on_youtube {
                        let _ = wv.eval(&injectable);
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            save_script,
            delete_script,
            save_settings,
            register_origin,
            add_preset,
            reset_scripts,
            install_from_url,
            open_youtube,
            open_search,
            open_home,
            reload_youtube
        ])
        .run(tauri::generate_context!())
        .expect("TubeScript başlatılamadı");
}
