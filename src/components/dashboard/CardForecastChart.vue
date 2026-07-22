<script setup lang="ts">
// Dashboard chart — projected upcoming card installments (forecast) as bars, with optional compact/table modes.
import { computed, onMounted, onUnmounted, ref } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { BarChart } from "echarts/charts";
import { GridComponent, TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { ForecastPoint } from "@/types/api.types";

use([CanvasRenderer, BarChart, GridComponent, TooltipComponent]);

const props = withDefaults(
  defineProps<{ points: ForecastPoint[]; compact?: boolean; table?: boolean }>(),
  { compact: false, table: false }
);
const totalAmount = computed(() => props.points.reduce((a, p) => a + n(p.amount), 0));

const isDark = ref(false);
const mql = window.matchMedia("(prefers-color-scheme: dark)");
const sync = () => (isDark.value = mql.matches);
onMounted(() => { sync(); mql.addEventListener("change", sync); });
onUnmounted(() => mql.removeEventListener("change", sync));

const MONTHS = ["Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez"];
const n = (s: string) => parseFloat(s) || 0;
const label = (ym: string) => {
  const [y, m] = ym.split("-");
  return `${MONTHS[parseInt(m, 10) - 1] ?? m}/${y.slice(2)}`;
};
const brlF = (v: number) => v.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
const brlShort = (v: number) => "R$ " + Math.round(v).toLocaleString("pt-BR");

const hasData = computed(() => props.points.some((p) => n(p.amount) > 0));

const option = computed(() => {
  const accent = isDark.value ? "#34c9a6" : "#0e7c66";
  const axis = isDark.value ? "#8aa39b" : "#5b6f68";
  const split = isDark.value ? "rgba(255,255,255,.08)" : "rgba(16,32,27,.08)";
  const tipBg = isDark.value ? "#14201d" : "#ffffff";
  const tipInk = isDark.value ? "#e8f0ed" : "#10201b";
  const pts = props.points;
  return {
    tooltip: {
      trigger: "axis",
      backgroundColor: tipBg,
      borderColor: split,
      borderWidth: 1,
      textStyle: { color: tipInk, fontSize: 12 },
      formatter: (ps: any[]) => {
        const i = ps[0].dataIndex;
        const p = pts[i];
        const rows = (p.items ?? [])
          .slice(0, 8)
          .map((it) => `<div style="display:flex;gap:12px;justify-content:space-between"><span>${it.description} <span style="opacity:.6">${it.parcela}</span></span><b>${brlF(n(it.amount))}</b></div>`)
          .join("");
        const more = (p.items?.length ?? 0) > 8 ? `<div style="opacity:.6">+${p.items.length - 8} mais…</div>` : "";
        return `<b>${label(p.month)}</b> · ${brlF(n(p.amount))}<div style="margin-top:4px;font-size:11.5px">${rows}${more}</div>`;
      },
    },
    grid: { left: 52, right: 14, top: 26, bottom: 26 },
    xAxis: {
      type: "category",
      data: pts.map((p) => label(p.month)),
      axisLabel: { color: axis, fontSize: 11, interval: props.compact ? "auto" : 0, rotate: pts.length > 10 ? 40 : 0 },
      axisLine: { lineStyle: { color: split } },
      axisTick: { show: false },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: axis, fontSize: 11, formatter: (v: number) => (v >= 1000 ? v / 1000 + "k" : "" + v) },
      splitLine: { lineStyle: { color: split } },
    },
    series: [
      {
        type: "bar",
        data: pts.map((p) => n(p.amount)),
        itemStyle: { color: accent, borderRadius: [4, 4, 0, 0] },
        barMaxWidth: 40,
        label: {
          show: true,
          position: "top",
          color: axis,
          fontSize: props.compact ? 10 : 11,
          fontWeight: 700,
          formatter: (p: any) => (p.value > 0 ? brlShort(p.value) : ""),
        },
      },
    ],
  };
});
</script>

<template>
  <div v-if="hasData" class="fc-wrap" :class="{ split: table }">
    <VChart :option="option" autoresize :style="{ height: compact ? '200px' : '300px' }" class="fc-chart" />
    <div v-if="table" class="fc-table" role="table" aria-label="Valores a pagar por mês">
      <div class="fc-row head"><span>Mês</span><span>A pagar</span></div>
      <div v-for="p in points" :key="p.month" class="fc-row">
        <span>{{ label(p.month) }}</span><b>{{ brlF(n(p.amount)) }}</b>
      </div>
      <div class="fc-row tot"><span>Total</span><b>{{ brlF(totalAmount) }}</b></div>
    </div>
  </div>
  <p v-else class="fc-empty">Sem parcelas futuras — nenhum compromisso de cartão à frente.</p>
</template>

<style scoped>
.fc-empty { font-size: 13px; color: var(--clr-text-muted, #7c8b83); margin: 0; padding: 10px 0; }
.fc-wrap.split { display: flex; gap: 18px; align-items: stretch; }
.fc-wrap.split .fc-chart { flex: 1; min-width: 0; }
.fc-table { width: 240px; flex: none; overflow-y: auto; max-height: 300px; font-size: 13px; font-variant-numeric: tabular-nums;
  border: 1px solid var(--clr-stroke); border-radius: 10px; align-self: flex-start; }
.fc-row { display: flex; justify-content: space-between; gap: 12px; padding: 8px 12px; border-bottom: 1px solid var(--clr-stroke-soft, var(--clr-stroke)); }
.fc-row:last-child { border-bottom: none; }
.fc-row.head { font-size: 11px; text-transform: uppercase; letter-spacing: .03em; color: var(--clr-text-muted, #7c8b83); font-weight: 700; background: var(--clr-surface-alt, transparent); position: sticky; top: 0; }
.fc-row span { color: var(--clr-text-secondary); } .fc-row b { color: var(--clr-text-primary); font-weight: 700; }
.fc-row.tot { font-weight: 800; background: var(--clr-surface-alt, transparent); }
.fc-row.tot span, .fc-row.tot b { color: var(--clr-text-primary); }
@media (max-width: 720px) { .fc-wrap.split { flex-direction: column; } .fc-table { width: 100%; max-height: none; } }
</style>
