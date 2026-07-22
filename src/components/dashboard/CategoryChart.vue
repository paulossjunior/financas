<script setup lang="ts">
// Dashboard pie chart of spend by category.
import { computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { PieChart } from "echarts/charts";
import { TitleComponent, TooltipComponent, LegendComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import type { Category } from "@/types/api.types";

use([CanvasRenderer, PieChart, TitleComponent, TooltipComponent, LegendComponent]);

const props = defineProps<{ categories: Category[] }>();

const FLUENT_COLORS = ['#0078d4','#00b7c3','#8764b8','#e3008c','#bad80a','#00bcf2','#ff8c00','#e81123'];

const option = computed(() => ({
  color: FLUENT_COLORS,
  tooltip: {
    trigger: "item",
    backgroundColor: "rgba(255,255,255,0.96)",
    borderColor: "#e0e0e0",
    borderWidth: 1,
    textStyle: { color: "#201f1e", fontSize: 12 },
    formatter: (params: any) => {
      const pct = params.data.percentage.toFixed(1);
      const amt = parseFloat(params.data.value).toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
      return `<b>${params.name}</b><br/>${amt} · ${pct}%`;
    },
  },
  legend: {
    orient: "vertical",
    left: "left",
    type: "scroll",
    textStyle: { color: "#605e5c", fontSize: 12 },
    itemWidth: 10,
    itemHeight: 10,
  },
  series: [
    {
      name: "Gastos",
      type: "pie",
      radius: ["42%", "72%"],
      center: ["60%", "50%"],
      avoidLabelOverlap: true,
      label: { show: false },
      emphasis: {
        label: { show: true, fontSize: 13, fontWeight: "bold", color: "#201f1e" },
        itemStyle: { shadowBlur: 8, shadowColor: "rgba(0,0,0,0.15)" },
      },
      data: props.categories.map((c) => ({
        name: c.name,
        value: c.net_total,
        percentage: c.percentage,
      })),
    },
  ],
}));
</script>

<template>
  <div class="chart-container">
    <h3>Distribuição por Categoria</h3>
    <VChart :option="option" autoresize style="height: 350px" />
  </div>
</template>

<style scoped>
.chart-container h3 { font-size: 0.875rem; font-weight: 600; margin-bottom: 0.75rem; color: var(--clr-text-primary); letter-spacing: -0.005em; }
</style>
