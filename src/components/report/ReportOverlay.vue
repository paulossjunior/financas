<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import reportCss from "@/assets/report.css?raw";

defineProps<{ title: string }>();
const emit = defineEmits<{ (e: "close"): void }>();

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));

const busy = ref(false);

// Light-theme tokens + layout so the standalone page renders faithfully in a browser.
const STANDALONE_CSS = `
:root{--clr-bg:#f4f6f5;--clr-surface:#fff;--clr-surface-alt:#eef1f0;--clr-stroke:#dde3e1;--clr-stroke-soft:#e7ecea;
--clr-text-primary:#16211e;--clr-text-secondary:#4a5a55;--clr-text-muted:#7c8b86;--clr-accent:#0e7c66;
--clr-accent-light:#d3ebe4;--clr-amber:#b45309;--clr-red-soft:#f6d9d9;--clr-negative:#b91c1c;--clr-track:#e7ecea;}
*{box-sizing:border-box;}
body{margin:0;background:#fff;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",system-ui,sans-serif;-webkit-print-color-adjust:exact;print-color-adjust:exact;}
.report-overlay{background:#fff;}
.report-doc{max-width:940px;margin:0 auto;padding:24px 20px;}
@media print{.report-doc{max-width:none;padding:0;}}
`;

// window.print() is unreliable inside the macOS WKWebview (often a no-op). The robust path:
// serialize the rendered report to a standalone HTML file (canvases → PNG) and open it in the
// system browser, where Ctrl/Cmd+P → "Salvar como PDF" always works. Fallback to window.print().
async function exportPdf() {
  if (busy.value) return;
  busy.value = true;
  try {
    const doc = document.querySelector<HTMLElement>(".report-overlay .report-doc");
    if (!doc) { window.print(); return; }
    const clone = doc.cloneNode(true) as HTMLElement;
    // Replace live ECharts canvases with static images so they survive serialization.
    const srcCanvas = doc.querySelectorAll("canvas");
    const dstCanvas = clone.querySelectorAll("canvas");
    srcCanvas.forEach((cv, i) => {
      try {
        const img = document.createElement("img");
        img.src = (cv as HTMLCanvasElement).toDataURL("image/png");
        img.style.width = (cv.clientWidth || 600) + "px";
        img.style.height = (cv.clientHeight || 320) + "px";
        dstCanvas[i]?.replaceWith(img);
      } catch { /* tainted canvas — leave as is */ }
    });
    const html =
      `<!doctype html><html lang="pt-BR"><head><meta charset="utf-8">` +
      `<meta name="viewport" content="width=device-width,initial-scale=1">` +
      `<title>Relatório — Finanças</title><style>${STANDALONE_CSS}${reportCss}</style></head>` +
      `<body><div class="report-overlay"><div class="report-doc">${clone.innerHTML}</div></div>` +
      `<script>window.addEventListener("load",function(){setTimeout(function(){window.print();},400);});<\/script>` +
      `</body></html>`;

    const [{ writeTextFile, BaseDirectory }, { appDataDir, join }, { openPath }] = await Promise.all([
      import("@tauri-apps/plugin-fs"),
      import("@tauri-apps/api/path"),
      import("@tauri-apps/plugin-opener"),
    ]);
    const file = "relatorio-financas.html";
    await writeTextFile(file, html, { baseDir: BaseDirectory.AppData });
    const full = await join(await appDataDir(), file);
    await openPath(full);
  } catch (e) {
    // Non-mac webviews (WebView2 / WebKitGTK) print fine in-app.
    console.error("export falhou, usando window.print():", e);
    window.print();
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="report-overlay" role="dialog" aria-modal="true" :aria-label="title">
      <div class="report-toolbar">
        <span class="rt-title">{{ title }}</span>
        <div class="rt-actions">
          <button class="rt-btn primary" :disabled="busy" @click="exportPdf">🖨 {{ busy ? "Gerando…" : "Exportar PDF" }}</button>
          <button class="rt-btn" @click="emit('close')">Fechar ✕</button>
        </div>
      </div>
      <div class="report-doc">
        <slot />
      </div>
    </div>
  </Teleport>
</template>

<style src="@/assets/report.css"></style>

<style scoped>
.report-overlay {
  position: fixed;
  inset: 0;
  z-index: 1000;
  overflow-y: auto;
  background: var(--clr-bg);
  padding: 0 0 60px;
}
.report-toolbar {
  position: sticky;
  top: 0;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 20px;
  background: var(--surface, #fff);
  border-bottom: 1px solid var(--line);
  box-shadow: 0 1px 3px rgba(0, 0, 0, .06);
}
.rt-title { font-weight: 800; font-size: 15px; color: var(--ink); }
.rt-actions { margin-left: auto; display: flex; gap: 10px; }
.rt-btn {
  font-family: inherit; font-size: 13px; font-weight: 700; padding: 8px 16px;
  border-radius: 8px; border: 1px solid var(--line); background: var(--track); color: var(--ink-2); cursor: pointer;
}
.rt-btn:hover { background: var(--line); }
.rt-btn.primary { background: var(--accent); color: #fff; border-color: var(--accent); }
.rt-btn.primary:hover { filter: brightness(1.05); }
.report-doc { max-width: 940px; margin: 24px auto 0; padding: 0 20px; }
</style>
