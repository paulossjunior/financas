<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getInflation, fetchIpca } from "@/services/tauri.service";
import type { InflationData } from "@/types/api.types";

withDefaults(defineProps<{ compact?: boolean }>(), { compact: false });

const data = ref<InflationData | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

const num = (s?: string) => parseFloat(s ?? "0") || 0;
const pct = (s?: string) =>
  num(s).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + "%";
const diffLabel = (s?: string) => {
  const v = num(s);
  const sign = v > 0 ? "+" : v < 0 ? "−" : "";
  return `${sign}${Math.abs(v).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 })} p.p.`;
};

const MM = ["jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez"];
const available = computed(() => !!data.value?.available);
const headline = computed(() => data.value?.headline ?? null);
const refLabel = computed(() => {
  const rm = headline.value?.ref_month;
  if (!rm) return "";
  const [y, m] = rm.split("-");
  return `${MM[parseInt(m, 10) - 1] ?? m}/${y}`;
});
const diffPositive = computed(() => num(data.value?.personal_diff) > 0);

async function load() {
  try {
    data.value = await getInflation();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}
async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    data.value = await fetchIpca();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}
onMounted(load);
</script>

<template>
  <div class="infl">
    <div class="infl-head">
      <div>
        <h3>Inflação</h3>
        <p class="cap" v-if="available">
          IPCA · ref. {{ refLabel }}<span v-if="data?.fetched_at"> · atualizado {{ data.fetched_at }}</span>
        </p>
        <p class="cap" v-else>IPCA oficial + sua inflação pessoal.</p>
      </div>
      <button class="upd" :disabled="loading" @click="refresh">
        {{ loading ? "Atualizando…" : "↻ Atualizar índices" }}
      </button>
    </div>

    <p v-if="error" class="err">⚠ {{ error }}</p>

    <template v-if="available && headline">
      <div class="kpis">
        <div class="k"><span class="l">Sua inflação (mês)</span><span class="v" :class="diffPositive ? 'up' : 'down'">{{ pct(data?.personal_month) }}</span><span class="s">{{ diffLabel(data?.personal_diff) }} vs IPCA</span></div>
        <div class="k"><span class="l">IPCA mês</span><span class="v">{{ pct(headline.month) }}</span></div>
        <template v-if="!compact">
          <div class="k"><span class="l">IPCA no ano</span><span class="v">{{ pct(headline.year) }}</span></div>
          <div class="k"><span class="l">IPCA 12 meses</span><span class="v">{{ pct(headline.twelve) }}</span></div>
        </template>
      </div>
      <p class="note" v-if="!compact">
        Sua inflação repondera os grupos do IPCA pelo peso dos seus gastos. Acima do IPCA = sua cesta subiu mais que a média.
      </p>
    </template>

    <div v-else-if="!error" class="empty">
      <p>Sem índices ainda. Clique em <b>Atualizar índices</b> para baixar o IPCA do IBGE (fica salvo, funciona offline depois).</p>
    </div>
  </div>
</template>

<style scoped>
.infl { display: flex; flex-direction: column; gap: 12px; }
.infl-head { display: flex; align-items: flex-start; gap: 12px; }
.infl-head h3 { margin: 0; font-size: 14.5px; font-weight: 800; }
.cap { margin: 2px 0 0; font-size: 12px; color: var(--clr-text-muted, #7c8b83); }
.upd { margin-left: auto; flex: none; font-family: inherit; font-size: 12.5px; font-weight: 700; padding: 7px 13px;
  border-radius: 9px; border: 1px solid var(--clr-stroke); background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer; }
.upd:hover { border-color: var(--clr-accent); color: var(--clr-accent); }
.upd:disabled { opacity: .6; cursor: default; }
.err { margin: 0; font-size: 13px; color: var(--clr-negative); }
.kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(120px, 1fr)); gap: 12px; }
.k { border: 1px solid var(--clr-stroke); border-radius: 11px; padding: 11px 13px; display: flex; flex-direction: column; gap: 2px; }
.k .l { font-size: 11.5px; color: var(--clr-text-secondary); font-weight: 600; }
.k .v { font-size: 21px; font-weight: 800; letter-spacing: -.02em; font-variant-numeric: tabular-nums; }
.k .v.up { color: var(--clr-negative); }
.k .v.down { color: var(--clr-accent); }
.k .s { font-size: 11px; color: var(--clr-text-muted, #7c8b83); font-variant-numeric: tabular-nums; }
.note { margin: 0; font-size: 12px; color: var(--clr-text-muted, #7c8b83); }
.empty { font-size: 13px; color: var(--clr-text-secondary); }
.empty p { margin: 0; }
</style>
