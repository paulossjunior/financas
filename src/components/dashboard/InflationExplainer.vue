<script setup lang="ts">
// Dashboard plain-language explainer of personal inflation relative to income/expense.
import { computed } from "vue";
import type { InflationData } from "@/types/api.types";
import { buildExplainer } from "@/utils/inflation-explainer";

const props = withDefaults(
  defineProps<{ data: InflationData | null; monthlyExpense: number; monthlyIncome: number; compact?: boolean }>(),
  { compact: false }
);

const num = (s?: string) => parseFloat(s ?? "0") || 0;

// Group pushing inflation up the most (for the "puxada por…" phrase).
const topGroup = computed(() => {
  const gs = props.data?.groups ?? [];
  if (!gs.length) return undefined;
  return [...gs].sort((a, b) => num(b.month_var) - num(a.month_var))[0]?.name;
});

const explainer = computed(() => {
  const d = props.data;
  if (!d?.available || !d.headline) return null;
  return buildExplainer({
    inflAnnual: num(d.headline.twelve),
    personalMonth: num(d.personal_month),
    personalDiff: num(d.personal_diff),
    monthlyExpense: props.monthlyExpense,
    monthlyIncome: props.monthlyIncome,
    topGroup: topGroup.value,
  });
});

const items = computed(() => {
  const its = explainer.value?.items ?? [];
  return props.compact ? its.slice(0, 2) : its;
});
</script>

<template>
  <div class="expl">
    <div class="expl-head">
      <h3>Entenda a inflação no seu bolso</h3>
      <p class="cap">Traduz o IPCA e a sua inflação em reais. Projeções são estimativas “se continuar assim”.</p>
    </div>

    <div v-if="items.length" class="cards">
      <div v-for="it in items" :key="it.id" class="xc">
        <div class="xt">{{ it.title }}</div>
        <div class="xb" :class="it.tone">{{ it.big }}</div>
        <p class="xp">{{ it.phrase }}</p>
        <div v-if="it.proj" class="xproj">
          <div v-for="p in it.proj" :key="p.label" class="xrow">
            <span>{{ p.label }}</span><b>{{ p.cost }}</b><span class="ex">{{ p.extra }}</span>
          </div>
        </div>
      </div>
    </div>
    <p v-else class="empty">Atualize os índices para ver o impacto da inflação nos seus números.</p>
  </div>
</template>

<style scoped>
.expl { display: flex; flex-direction: column; gap: 12px; }
.expl-head h3 { margin: 0; font-size: 14.5px; font-weight: 800; }
.cap { margin: 2px 0 0; font-size: 12px; color: var(--clr-text-muted, #7c8b83); }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 12px; }
.xc { border: 1px solid var(--clr-stroke); border-radius: 12px; padding: 14px 15px; display: flex; flex-direction: column; gap: 4px; }
.xt { font-size: 11.5px; font-weight: 700; color: var(--clr-text-secondary); text-transform: uppercase; letter-spacing: .02em; }
.xb { font-size: 24px; font-weight: 800; letter-spacing: -.02em; font-variant-numeric: tabular-nums; }
.xb.bad { color: var(--clr-negative); } .xb.good { color: var(--clr-accent); } .xb.neutral { color: var(--clr-text-primary); }
.xp { margin: 2px 0 0; font-size: 12.5px; color: var(--clr-text-secondary); line-height: 1.5; }
.xproj { margin-top: 8px; border-top: 1px solid var(--clr-stroke-soft, var(--clr-stroke)); padding-top: 8px; display: grid; gap: 4px; }
.xrow { display: grid; grid-template-columns: 64px 1fr auto; gap: 8px; font-size: 12.5px; font-variant-numeric: tabular-nums; }
.xrow span { color: var(--clr-text-muted, #7c8b83); } .xrow b { color: var(--clr-text-primary); text-align: right; }
.xrow .ex { color: var(--clr-negative); text-align: right; }
.empty { font-size: 13px; color: var(--clr-text-secondary); margin: 0; }
</style>
