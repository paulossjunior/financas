<script setup lang="ts">
// Password prompt for encrypted invoice files (BTG .xlsx, Santander .pdf).
// Pure UI: the parent owns the import call and feeds errors back via props.
import { ref, watch } from "vue";

const props = defineProps<{
  open: boolean;
  loading: boolean;
  error: string | null;
  /** Which bank's password is being asked ("BTG", "Santander") — banks differ. */
  bank: string | null;
}>();

const emit = defineEmits<{
  submit: [password: string, remember: boolean];
  cancel: [];
}>();

const password = ref("");
const remember = ref(true);

// A fresh prompt starts clean; keep the typed value while the same prompt retries.
watch(
  () => props.open,
  (open) => {
    if (open) password.value = "";
  }
);

function submit(): void {
  if (!password.value || props.loading) return;
  emit("submit", password.value, remember.value);
}
</script>

<template>
  <div v-if="props.open" class="pw-overlay" @click.self="emit('cancel')">
    <div class="pw-modal" role="dialog" aria-modal="true" aria-label="Fatura protegida por senha">
      <h3 data-testid="pw-title">
        Senha da fatura {{ props.bank ?? "" }}
      </h3>
      <p class="pw-sub">
        {{ props.bank ? `Este arquivo do ${props.bank} está criptografado.` : "Este arquivo está criptografado." }}
        Informe a senha para importar — cada banco tem a sua.
      </p>
      <input
        v-model="password"
        type="password"
        class="pw-input"
        :placeholder="props.bank ? `Senha da fatura ${props.bank}` : 'Senha do arquivo'"
        autofocus
        data-testid="pw-input"
        @keyup.enter="submit"
        @keyup.esc="emit('cancel')"
      />
      <label class="pw-remember">
        <input type="checkbox" v-model="remember" />
        <span>Lembrar {{ props.bank ? `a senha do ${props.bank}` : "senha" }} neste dispositivo (Keychain)</span>
      </label>
      <div v-if="props.error" class="pw-err" data-testid="pw-error">⚠ {{ props.error }}</div>
      <div class="pw-actions">
        <button class="pw-btn ghost" @click="emit('cancel')">Cancelar</button>
        <button
          class="pw-btn primary"
          data-testid="pw-submit"
          :disabled="props.loading || !password"
          @click="submit"
        >
          {{ props.loading ? "Abrindo…" : "Importar" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pw-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 60;
}
.pw-modal {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-md, 0 8px 30px rgba(0, 0, 0, 0.18));
  padding: 24px;
  width: min(420px, 92vw);
}
.pw-modal h3 { font-size: 17px; font-weight: 800; color: var(--clr-text-primary); margin: 0 0 6px; }
.pw-sub { font-size: 13px; color: var(--clr-text-secondary); margin: 0 0 14px; }
.pw-input {
  width: 100%;
  padding: 0.55rem 0.7rem;
  font: inherit;
  font-size: 0.9rem;
  color: var(--clr-text-primary);
  background: var(--clr-bg);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-sm);
  outline: none;
}
.pw-input:focus { border-color: var(--clr-accent); }
.pw-remember {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
  font-size: 13px;
  color: var(--clr-text-secondary);
  cursor: pointer;
  user-select: none;
}
.pw-remember input { width: 15px; height: 15px; accent-color: var(--clr-accent); cursor: pointer; }
.pw-err { margin-top: 10px; font-size: 13px; color: var(--clr-negative); }
.pw-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
.pw-btn {
  font: inherit;
  font-size: 0.8125rem;
  font-weight: 600;
  border-radius: var(--radius-md);
  padding: 0.45rem 1rem;
  cursor: pointer;
  border: 1px solid var(--clr-stroke);
  background: var(--clr-surface);
  color: var(--clr-text-secondary);
}
.pw-btn.primary {
  background: var(--clr-accent);
  border-color: transparent;
  color: #fff;
}
.pw-btn.primary:disabled { opacity: 0.5; cursor: not-allowed; }
.pw-btn.ghost:hover { background: var(--clr-surface-alt); color: var(--clr-text-primary); }
</style>
