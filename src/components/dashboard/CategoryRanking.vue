<script setup lang="ts">
// Dashboard horizontal bar chart ranking categories by net total.
import { computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { BarChart } from "echarts/charts";
import { GridComponent, TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { Category } from "@/types/api.types";

use([CanvasRenderer, BarChart, GridComponent, TooltipComponent]);

const props = defineProps<{ categories: Category[] }>();

const option = computed(() => {
  const sorted = [...props.categories].sort(
    (a, b) => parseFloat(a.net_total) - parseFloat(b.net_total)
  );
  return {
    tooltip: {
      trigger: "axis",
      backgroundColor: "rgba(255,255,255,0.96)",
      borderColor: "#e0e0e0",
      borderWidth: 1,
      textStyle: { color: "#201f1e", fontSize: 12 },
      formatter: (params: any) => {
        const p = params[0];
        const amt = parseFloat(p.value).toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
        return `<b>${p.name}</b><br/>${amt}`;
      },
    },
    grid: { left: "140px", right: "24px", top: "8px", bottom: "8px" },
    xAxis: {
      type: "value",
      axisLabel: {
        formatter: (v: number) => `R$ ${v.toLocaleString("pt-BR")}`,
        color: "#a19f9d",
        fontSize: 11,
      },
      splitLine: { lineStyle: { color: "#ebebeb" } },
    },
    yAxis: {
      type: "category",
      data: sorted.map((c) => c.name),
      axisLabel: { width: 120, overflow: "truncate", color: "#605e5c", fontSize: 12 },
      axisLine: { show: false },
      axisTick: { show: false },
    },
    series: [
      {
        type: "bar",
        data: sorted.map((c, i) => ({
          value: c.net_total,
          itemStyle: {
            color: i === sorted.length - 1 ? "#0078d4" : "#c7e0f4",
            borderRadius: [0, 3, 3, 0],
          },
        })),
        barMaxWidth: 32,
      },
    ],
  };
});
</script>

<template>
  <div class="chart-container">
    <h3>Ranking de Categorias</h3>
    <VChart :option="option" autoresize style="height: 300px" />
  </div>
</template>

<style scoped>
.chart-container h3 { font-size: 0.875rem; font-weight: 600; margin-bottom: 0.75rem; color: var(--clr-text-primary); letter-spacing: -0.005em; }
</style>
