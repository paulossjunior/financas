<script setup lang="ts">
import { RouterView, RouterLink } from "vue-router";
import { ref, onErrorCaptured } from "vue";

const globalError = ref<string | null>(null);

const ERROR_MESSAGES: Record<string, string> = {
  ENCRYPTED_FILE: "Arquivo protegido por senha. Abra no Excel ou Numbers, remova a proteção e salve novamente.",
  FILE_NOT_FOUND: "Arquivo não encontrado. Verifique se o caminho está correto.",
  NO_DATA: "Nenhuma fatura importada. Importe uma fatura BTG para começar.",
  INVALID_FORMAT: "Formato de arquivo inválido. Verifique se é uma fatura BTG válida.",
  PARSE_ERROR: "Erro ao processar o arquivo. Verifique se o arquivo não está corrompido.",
};

function toPortuguese(raw: string): string {
  if (!raw) return "Erro desconhecido. Tente novamente.";
  for (const [code, msg] of Object.entries(ERROR_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.startsWith("INVALID_FORMAT:")) return `${ERROR_MESSAGES.INVALID_FORMAT} Detalhe: ${raw.slice(15)}`;
  if (raw.startsWith("PARSE_ERROR:")) return `${ERROR_MESSAGES.PARSE_ERROR} Detalhe: ${raw.slice(12)}`;
  return "Ocorreu um erro inesperado. Por favor, tente novamente.";
}

function showError(raw: string): void {
  const msg = toPortuguese(raw);
  globalError.value = msg;
  setTimeout(() => (globalError.value = null), 6000);
}

window.addEventListener("unhandledrejection", (event) => {
  const raw = event.reason instanceof Error ? event.reason.message : String(event.reason ?? "");
  if (raw && !raw.includes("NO_DATA")) {
    showError(raw);
  }
});

onErrorCaptured((err) => {
  const raw = err instanceof Error ? err.message : String(err);
  showError(raw);
  return false;
});
</script>

<template>
  <div class="app">
    <nav class="nav">
      <RouterLink to="/" active-class="active">📊 Dashboard</RouterLink>
      <RouterLink to="/transacoes" active-class="active">🧾 Despesas</RouterLink>
      <RouterLink to="/historico" active-class="active">📈 Histórico</RouterLink>
      <RouterLink to="/configuracoes" active-class="active">⚙️ Configurações</RouterLink>
    </nav>

    <div v-if="globalError" class="global-error" role="alert">
      <span>⚠️ {{ globalError }}</span>
      <button class="dismiss" @click="globalError = null" aria-label="Fechar">✕</button>
    </div>

    <main>
      <RouterView />
    </main>
  </div>
</template>

<style>
  /* ── Fluent Design Tokens ── */
  :root {
    --clr-bg:          #f3f3f3;
    --clr-surface:     #ffffff;
    --clr-surface-alt: #fafafa;
    --clr-stroke:      #e0e0e0;
    --clr-stroke-soft: #ebebeb;

    --clr-text-primary:   #201f1e;
    --clr-text-secondary: #605e5c;
    --clr-text-muted:     #a19f9d;

    --clr-accent:        #0078d4;
    --clr-accent-hover:  #106ebe;
    --clr-accent-light:  #eff6fc;

    --clr-positive:      #107c10;
    --clr-negative:      #a4262c;
    --clr-warning:       #797673;

    --clr-chart-1: #0078d4;
    --clr-chart-2: #00b7c3;
    --clr-chart-3: #8764b8;
    --clr-chart-4: #e3008c;
    --clr-chart-5: #bad80a;
    --clr-chart-6: #00bcf2;
    --clr-chart-7: #ff8c00;
    --clr-chart-8: #e81123;

    --radius-sm:  4px;
    --radius-md:  8px;
    --radius-lg:  12px;

    --shadow-sm:  0 1px 2px rgba(0,0,0,.06), 0 1px 4px rgba(0,0,0,.04);
    --shadow-md:  0 2px 4px rgba(0,0,0,.08), 0 4px 12px rgba(0,0,0,.06);
    --shadow-lg:  0 4px 8px rgba(0,0,0,.10), 0 8px 24px rgba(0,0,0,.08);

    --font-body: "Segoe UI", system-ui, -apple-system, sans-serif;
  }

  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  body {
    font-family: var(--font-body);
    font-size: 14px;
    background: var(--clr-bg);
    color: var(--clr-text-primary);
    -webkit-font-smoothing: antialiased;
  }

  /* Fluent scrollbar */
  ::-webkit-scrollbar { width: 6px; height: 6px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: #c8c6c4; border-radius: 3px; }
  ::-webkit-scrollbar-thumb:hover { background: #a19f9d; }
</style>

<style scoped>
.app { min-height: 100vh; display: flex; flex-direction: column; }

/* Fluent acrylic nav */
.nav {
  background: rgba(255,255,255,0.85);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-bottom: 1px solid var(--clr-stroke);
  padding: 0 1.5rem;
  display: flex;
  gap: 0;
  position: sticky;
  top: 0;
  z-index: 100;
}
.nav a {
  padding: 0.9rem 1.1rem;
  text-decoration: none;
  color: var(--clr-text-secondary);
  font-size: 0.875rem;
  font-weight: 400;
  border-bottom: 2px solid transparent;
  transition: color 0.1s, border-color 0.1s, background 0.1s;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.nav a:hover {
  color: var(--clr-text-primary);
  background: rgba(0,0,0,0.04);
}
.nav a.active {
  color: var(--clr-accent);
  border-bottom-color: var(--clr-accent);
  font-weight: 600;
}

/* Fluent message bar (error) */
.global-error {
  background: #fde7e9;
  border-bottom: 1px solid #f1707b;
  color: var(--clr-negative);
  padding: 0.65rem 1.5rem;
  font-size: 0.8125rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}
.dismiss {
  background: none;
  border: none;
  color: var(--clr-negative);
  cursor: pointer;
  font-size: 0.875rem;
  padding: 0.25rem;
  border-radius: var(--radius-sm);
  flex-shrink: 0;
  line-height: 1;
}
.dismiss:hover { background: rgba(164,38,44,0.08); }

main { flex: 1; }
</style>
