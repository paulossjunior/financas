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
      <div class="nav-inner">
        <div class="brand"><span class="brand-mark">◍</span> Finanças</div>
        <div class="tabs">
          <RouterLink to="/" active-class="active"><span class="ic">📊</span>Dashboard</RouterLink>
          <RouterLink to="/ano" active-class="active"><span class="ic">📅</span>Ano</RouterLink>
          <RouterLink to="/transacoes" active-class="active"><span class="ic">🧾</span>Despesas</RouterLink>
          <RouterLink to="/receitas-fixos" active-class="active"><span class="ic">💰</span>Receitas &amp; Fixos</RouterLink>
          <RouterLink to="/contracheque" active-class="active"><span class="ic">📄</span>Contracheque</RouterLink>
          <RouterLink to="/mapeamento" active-class="active"><span class="ic">🗂️</span>Mapeamento</RouterLink>
          <RouterLink to="/historico" active-class="active"><span class="ic">📈</span>Histórico</RouterLink>
          <RouterLink to="/configuracoes" active-class="active"><span class="ic">⚙️</span>Configurações</RouterLink>
        </div>
      </div>
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
  /* ── Emerald design tokens (exact artifact palette) ── */
  :root {
    --clr-bg:          #f4f6f5;
    --clr-surface:     #ffffff;
    --clr-surface-alt: #eef1f0;
    --clr-stroke:      #dde3e1;
    --clr-stroke-soft: #e7ecea;

    --clr-text-primary:   #16211e;
    --clr-text-secondary: #4a5a55;
    --clr-text-muted:     #7c8b86;

    --clr-accent:        #0e7c66;
    --clr-accent-hover:  #0b6353;
    --clr-accent-light:  #d3ebe4;

    --clr-positive:      #0e7c66;
    --clr-negative:      #b91c1c;
    --clr-warning:       #b45309;

    --clr-amber:         #b45309;
    --clr-amber-soft:    #f7e6cf;
    --clr-red-soft:      #f6d9d9;
    --clr-track:         #e7ecea;

    --clr-chart-1: #0e7c66;
    --clr-chart-2: #0ea5a0;
    --clr-chart-3: #6d4aff;
    --clr-chart-4: #b45309;
    --clr-chart-5: #d4a72c;
    --clr-chart-6: #2b7a78;
    --clr-chart-7: #c026a6;
    --clr-chart-8: #b91c1c;

    --radius-sm:  5px;
    --radius-md:  8px;
    --radius-lg:  14px;

    --shadow-sm:  0 1px 2px rgba(20,33,30,.05), 0 6px 20px rgba(20,33,30,.05);
    --shadow-md:  0 2px 6px rgba(20,33,30,.08), 0 10px 28px rgba(20,33,30,.07);
    --shadow-lg:  0 4px 10px rgba(20,33,30,.10), 0 16px 40px rgba(20,33,30,.10);

    --font-body: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      --clr-bg:          #0d1513;
      --clr-surface:     #14201d;
      --clr-surface-alt: #1a2825;
      --clr-stroke:      #263531;
      --clr-stroke-soft: #21302c;

      --clr-text-primary:   #e8efec;
      --clr-text-secondary: #a9b8b3;
      --clr-text-muted:     #7a8a85;

      --clr-accent:        #34c9a6;
      --clr-accent-hover:  #46d9b6;
      --clr-accent-light:  #123d34;

      --clr-positive:      #34c9a6;
      --clr-negative:      #f08a8a;
      --clr-warning:       #e0a458;

      --clr-amber:         #e0a458;
      --clr-amber-soft:    #3a2b12;
      --clr-red-soft:      #3a1c1c;
      --clr-track:         #21302c;

      --shadow-sm:  0 1px 2px rgba(0,0,0,.3), 0 6px 20px rgba(0,0,0,.35);
      --shadow-md:  0 2px 6px rgba(0,0,0,.4), 0 10px 28px rgba(0,0,0,.4);
      --shadow-lg:  0 4px 10px rgba(0,0,0,.45), 0 16px 40px rgba(0,0,0,.45);
    }
  }

  *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

  body {
    font-family: var(--font-body);
    font-size: 14px;
    background: var(--clr-bg);
    color: var(--clr-text-primary);
    -webkit-font-smoothing: antialiased;
    font-variant-numeric: tabular-nums;
  }

  ::-webkit-scrollbar { width: 8px; height: 8px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--clr-stroke); border-radius: 4px; }
  ::-webkit-scrollbar-thumb:hover { background: var(--clr-text-muted); }
</style>

<style scoped>
.app { min-height: 100vh; display: flex; flex-direction: column; }

/* Emerald top bar */
.nav {
  background: var(--clr-surface);
  border-bottom: 1px solid var(--clr-stroke);
  position: sticky;
  top: 0;
  z-index: 100;
  box-shadow: 0 1px 0 rgba(20,33,30,.02);
}
.nav-inner {
  max-width: 1320px;
  margin: 0 auto;
  padding: 0 2rem;
  display: flex;
  align-items: center;
  gap: 2rem;
  height: 60px;
}
.brand {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 1rem;
  font-weight: 800;
  letter-spacing: -0.01em;
  color: var(--clr-text-primary);
  white-space: nowrap;
}
.brand-mark {
  color: var(--clr-accent);
  font-size: 1.15rem;
  line-height: 1;
}
.tabs { display: flex; gap: 0.25rem; flex-wrap: wrap; }
.tabs a {
  padding: 0.5rem 0.85rem;
  text-decoration: none;
  color: var(--clr-text-secondary);
  font-size: 0.8125rem;
  font-weight: 600;
  border-radius: 100px;
  transition: color 0.12s, background 0.12s;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.tabs a .ic { font-size: 0.9rem; line-height: 1; }
.tabs a:hover {
  color: var(--clr-text-primary);
  background: var(--clr-surface-alt);
}
.tabs a.active {
  color: var(--clr-accent);
  background: var(--clr-accent-light);
  font-weight: 700;
}

/* Message bar (error) — emerald palette */
.global-error {
  background: var(--clr-red-soft);
  border-bottom: 1px solid var(--clr-negative);
  color: var(--clr-negative);
  padding: 0.65rem 2rem;
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
.dismiss:hover { background: rgba(185,28,28,0.1); }

main { flex: 1; }

@media (max-width: 720px) {
  .nav-inner { flex-direction: column; height: auto; padding: 0.6rem 1rem; gap: 0.5rem; align-items: flex-start; }
  .tabs { gap: 0.15rem; }
}
</style>
