<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { BarChart } from "echarts/charts";
import { GridComponent, TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { ForecastPoint } from "@/types/api.types";

use([CanvasRenderer, BarChart, GridComponent, TooltipComponent]);

const props = withDefaults(
  defineProps<{ points: ForecastPoint[]; compact?: boolean }>(),
  { compact: false }
);

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
    grid: { left: 52, right: 14, top: 14, bottom: 26 },
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
        barMaxWidth: 34,
      },
    ],
  };
});
</script>

<template>
  <VChart v-if="hasData" :option="option" autoresize :style="{ height: compact ? '200px' : '300px' }" />
  <p v-else class="fc-empty">Sem parcelas futuras — nenhum compromisso de cartão à frente.</p>
</template>

<style scoped>
.fc-empty { font-size: 13px; color: var(--clr-text-muted, #7c8b83); margin: 0; padding: 10px 0; }
</style>
