/* TubeScript - arayüz mantığı */
(() => {
  const tauri = window.__TAURI__;
  const invokeReal = tauri && tauri.core ? tauri.core.invoke : null;

  const invoke = async (cmd, args) => {
    if (!invokeReal) {
      console.warn("[TubeScript] Tauri yok, komut atlandı:", cmd, args);
      return null;
    }
    try {
      return await invokeReal(cmd, args);
    } catch (err) {
      console.error("[TubeScript] komut hatası:", cmd, err);
      toast("İşlem başarısız: " + (err && err.message ? err.message : err));
      throw err;
    }
  };

  const $ = (sel) => document.querySelector(sel);
  const $$ = (sel) => Array.from(document.querySelectorAll(sel));

  let state = { scripts: [], settings: {} };
  let editingId = null;

  // ---------------------------------------------------------------- yardımcılar
  let toastTimer = null;
  function toast(msg) {
    const el = $("#toast");
    el.textContent = msg;
    el.classList.add("show");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.classList.remove("show"), 2200);
  }

  function escapeHtml(str) {
    return String(str).replace(/[&<>"']/g, (c) => ({
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
    }[c]));
  }

  function showPage(name) {
    $$(".page").forEach((p) => p.classList.toggle("is-active", p.dataset.page === name));
    $$(".nav-item").forEach((b) => b.classList.toggle("is-active", b.dataset.nav === name));
    $("#content").scrollTo ? $("#content").scrollTo(0, 0) : window.scrollTo(0, 0);
  }

  // ---------------------------------------------------------------- render
  function render() {
    const scripts = state.scripts || [];
    const settings = state.settings || {};
    const enabled = scripts.filter((s) => s.enabled).length;

    // Ana sayfa chip'leri
    $("#home-chips").innerHTML = [
      `<span class="chip on">${scripts.length} script</span>`,
      `<span class="chip ${settings.adblock ? "on" : ""}">Reklam temizliği</span>`,
      `<span class="chip ${settings.hideShorts ? "on" : ""}">Shorts gizli</span>`,
      `<span class="chip ${settings.injectFab ? "on" : ""}">Yüzen ayarlar</span>`,
      `<span class="chip">microG</span>`
    ].join("");

    // Durum kartı
    const line = (label, value, on) =>
      `<div class="status-line"><span class="label"><i class="dot ${on ? "" : "off"}"></i>${label}</span><b>${value}</b></div>`;
    $("#home-status").innerHTML = [
      line("Etkin userscript", String(enabled), enabled > 0),
      line("Enjeksiyon hedefi", "m.youtube.com", true),
      line("Google Play Services", "gerekmez", true),
      line("Sürüm", "1.0.0", true)
    ].join("");

    // Script listesi
    $("#scripts-sub").textContent = `${scripts.length} script · ${enabled} etkin`;
    if (!scripts.length) {
      $("#script-list").innerHTML = `<div class="empty">Henüz script yok.<br>Sağ üstteki <b>+</b> ile ilk script'ini ekle.</div>`;
    } else {
      $("#script-list").innerHTML = scripts
        .map((s) => `
        <div class="script-item" data-id="${escapeHtml(s.id)}">
          <div class="meta">
            <strong>${escapeHtml(s.name)}</strong>
            <small>${escapeHtml((s.matches || []).join(", ") || "tüm sayfalar")}</small>
          </div>
          <input type="checkbox" class="switch script-toggle" data-id="${escapeHtml(s.id)}" ${s.enabled ? "checked" : ""} />
          <button class="edit" data-edit="${escapeHtml(s.id)}" title="Düzenle">✎</button>
        </div>`)
        .join("");
    }

    // Preset chip'leri
    $("#preset-row").innerHTML = [
      ["shorts", "Shorts gizle"],
      ["autoplay", "Otomatik oynatma kapat"],
      ["speed", "1.5x hız"],
      ["blank", "Boş script"]
    ]
      .map(([key, label]) => `<button class="chip" data-preset="${key}">+ ${label}</button>`)
      .join("");

    // Ayar anahtarları
    $$("[data-setting]").forEach((el) => {
      el.checked = !!settings[el.dataset.setting];
    });
    $("#origin-label").textContent = settings.localOrigin || location.origin;
  }

  function applyState(next) {
    if (!next) return;
    state = next;
    render();
  }

  // ---------------------------------------------------------------- script düzenleyici
  function openEditor(script) {
    editingId = script ? script.id : null;
    $("#editor-title").textContent = script ? "Script'i düzenle" : "Yeni userscript";
    $("#f-id").value = script ? script.id : "";
    $("#f-name").value = script ? script.name : "";
    $("#f-matches").value = script ? (script.matches || []).join(", ") : "https://m.youtube.com/*";
    $("#f-code").value = script
      ? script.code
      : "// userscript\nGM.addStyle('body{background:#000}');\nGM.log('çalıştı');";
    $("#f-enabled").checked = script ? !!script.enabled : true;
    $("#editor-delete").style.display = script ? "" : "none";
    const dlg = $("#editor");
    if (dlg.showModal) dlg.showModal(); else dlg.setAttribute("open", "");
  }

  function closeEditor() {
    const dlg = $("#editor");
    if (dlg.close) dlg.close(); else dlg.removeAttribute("open");
  }

  async function saveEditor() {
    const name = $("#f-name").value.trim();
    if (!name) return toast("Script adı gerekli");
    const matches = $("#f-matches").value
      .split(",")
      .map((m) => m.trim())
      .filter(Boolean);
    const existing = state.scripts.find((s) => s.id === editingId);
    const script = {
      id: editingId || "",
      name,
      matches: matches.length ? matches : ["https://m.youtube.com/*"],
      runAt: existing ? existing.runAt || "document-idle" : "document-idle",
      code: $("#f-code").value,
      enabled: $("#f-enabled").checked,
      updated: Date.now()
    };
    const next = await invoke("save_script", { script });
    applyState(next);
    closeEditor();
    toast("Script kaydedildi");
  }

  async function deleteEditing() {
    if (!editingId) return;
    if (!confirm("Bu script silinsin mi?")) return;
    const next = await invoke("delete_script", { id: editingId });
    applyState(next);
    closeEditor();
    toast("Script silindi");
  }

  // ---------------------------------------------------------------- ayarlar
  async function patchSetting(key, value) {
    const settings = { ...state.settings, [key]: value };
    const next = await invoke("save_settings", { settings });
    applyState(next);
    toast("Ayar kaydedildi");
  }

  // ---------------------------------------------------------------- olaylar
  function wire() {
    $$(".nav-item").forEach((btn) =>
      btn.addEventListener("click", () => showPage(btn.dataset.nav))
    );

    const openYt = async () => {
      toast("YouTube açılıyor…");
      await invoke("open_youtube");
    };
    $("#btn-open-top").addEventListener("click", openYt);
    $("#btn-open-hero").addEventListener("click", openYt);

    $("#btn-new-hero").addEventListener("click", () => {
      showPage("scripts");
      openEditor(null);
    });

    $("#search-form").addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const q = $("#search-input").value.trim();
      if (!q) return toast("Arama metni gerekli");
      toast(`"${q}" aranıyor…`);
      await invoke("open_search", { query: q });
    });
    $("#btn-new-script").addEventListener("click", () => openEditor(null));

    $("#script-list").addEventListener("click", async (ev) => {
      const editBtn = ev.target.closest("[data-edit]");
      if (editBtn) {
        const s = state.scripts.find((x) => x.id === editBtn.dataset.edit);
        if (s) openEditor(s);
      }
    });
    $("#script-list").addEventListener("change", async (ev) => {
      const toggle = ev.target.closest(".script-toggle");
      if (!toggle) return;
      const script = state.scripts.find((s) => s.id === toggle.dataset.id);
      if (!script) return;
      const next = await invoke("save_script", {
        script: { ...script, enabled: toggle.checked }
      });
      applyState(next);
    });

    $("#preset-row").addEventListener("click", async (ev) => {
      const chip = ev.target.closest("[data-preset]");
      if (!chip) return;
      const next = await invoke("add_preset", { preset: chip.dataset.preset });
      applyState(next);
      toast("Örnek script eklendi");
    });

    $("#install-form").addEventListener("submit", async (ev) => {
      ev.preventDefault();
      const url = $("#install-url").value.trim();
      if (!url) return toast("Script adresi gerekli");
      toast("İndiriliyor…");
      const next = await invoke("install_from_url", { url });
      if (next) {
        applyState(next);
        $("#install-url").value = "";
        toast("Script kuruldu");
      }
    });

    $$("[data-setting]").forEach((el) =>
      el.addEventListener("change", () => patchSetting(el.dataset.setting, el.checked))
    );

    $("#btn-reload-yt").addEventListener("click", async () => {
      await invoke("reload_youtube");
      toast("Filtreler yeniden uygulandı");
    });

    $("#btn-reset").addEventListener("click", async () => {
      if (!confirm("Tüm script'ler varsayılana dönecek. Devam?")) return;
      const next = await invoke("reset_scripts");
      applyState(next);
      toast("Script'ler sıfırlandı");
    });

    $("#editor-close").addEventListener("click", closeEditor);
    $("#editor-save").addEventListener("click", saveEditor);
    $("#editor-delete").addEventListener("click", deleteEditing);
    $("#editor-form").addEventListener("submit", (e) => e.preventDefault());
  }

  // ---------------------------------------------------------------- başlangıç
  async function boot() {
    wire();

    // Kabukta uzun basma menüsü / metin seçimi olmasın (giriş alanları hariç).
    document.addEventListener("contextmenu", (e) => {
      if (!e.target.closest("input, textarea")) e.preventDefault();
    });
    document.addEventListener("selectstart", (e) => {
      if (!e.target.closest("input, textarea")) e.preventDefault();
    });

    const params = new URLSearchParams(location.search);
    const view = params.get("view");
    if (view === "settings" || view === "scripts" || view === "home") showPage(view);

    try {
      await invoke("register_origin", { origin: location.origin });
      const loaded = await invoke("get_state");
      if (loaded) state = loaded;
    } catch (e) {
      console.error(e);
    }
    render();
  }

  boot();
})();
