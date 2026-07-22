<script setup lang="ts">
// Dashboard line chart of the monthly income/expense trend over time.
import { computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { LineChart } from "echarts/charts";
import { GridComponent, TooltipComponent, LegendComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { MonthlySnapshot } from "@/types/api.types";

use([CanvasRenderer, LineChart, GridComponent, TooltipComponent, LegendComponent]);

const props = defineProps<{ snapshots: MonthlySnapshot[] }>();

function formatMonth(ym: string): string {
  const [year, month] = ym.split("-");
  const months = ["Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez"];
  return `${months[parseInt(month, 10) - 1]}/${year}`;
}

const option = computed(() => {
  const xAxis = props.snapshots.map((s) => formatMonth(s.month));

  const categoryNames = [
    ...new Set(props.snapshots.flatMap((s) => s.categories.map((c) => c.name))),
  ];

  const series = categoryNames.map((name) => ({
    name,
    type: "line",
    smooth: true,
    data: props.snapshots.map((s) => {
      const cat = s.categories.find((c) => c.name === name);
      return cat ? parseFloat(cat.net_total) : 0;
    }),
  }));

  const FLUENT_COLORS = ['#0078d4','#00b7c3','#8764b8','#e3008c','#bad80a','#00bcf2','#ff8c00','#e81123'];
  return {
    color: FLUENT_COLORS,
    tooltip: {
      trigger: "axis",
      backgroundColor: "rgba(255,255,255,0.96)",
      borderColor: "#e0e0e0",
      borderWidth: 1,
      textStyle: { color: "#201f1e", fontSize: 12 },
    },
    legend: {
      data: categoryNames,
      top: "bottom",
      textStyle: { color: "#605e5c", fontSize: 12 },
      itemWidth: 12,
      itemHeight: 3,
    },
    grid: { left: "70px", right: "20px", top: "16px", bottom: "60px" },
    xAxis: {
      type: "category",
      data: xAxis,
      axisLabel: { color: "#605e5c", fontSize: 11 },
      axisLine: { lineStyle: { color: "#e0e0e0" } },
    },
    yAxis: {
      type: "value",
      axisLabel: { formatter: (v: number) => `R$ ${v.toLocaleString("pt-BR")}`, color: "#a19f9d", fontSize: 11 },
      splitLine: { lineStyle: { color: "#ebebeb" } },
    },
    series: series.map((s) => ({ ...s, lineStyle: { width: 2 }, symbol: "circle", symbolSize: 5 })),
  };
});
</script>

<template>
  <div class="chart-container">
    <h3>Evolução Mensal por Categoria</h3>
    <VChart :option="option" autoresize style="height: 380px" />
  </div>
</template>

<style scoped>
.chart-container h3 { font-size: 0.875rem; font-weight: 600; margin-bottom: 0.75rem; color: var(--clr-text-primary); letter-spacing: -0.005em; }
</style>
