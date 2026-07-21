<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";

defineProps<{ title: string }>();
const emit = defineEmits<{ (e: "close"): void }>();

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") emit("close");
}
onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));

function print() {
  window.print();
}
</script>

<template>
  <Teleport to="body">
    <div class="report-overlay" role="dialog" aria-modal="true" :aria-label="title">
      <div class="report-toolbar">
        <span class="rt-title">{{ title }}</span>
        <div class="rt-actions">
          <button class="rt-btn primary" @click="print">🖨 Exportar PDF / Imprimir</button>
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
