<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { TreemapChart } from "echarts/charts";
import { TooltipComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";

use([CanvasRenderer, TreemapChart, TooltipComponent]);

const props = withDefaults(
  defineProps<{ items: { name: string; value: number }[]; height?: string }>(),
  { height: "320px" }
);

// Category color palette (app tokens + a few extras), assigned in descending order.
const PALETTE = [
  "#0e7c66", "#0ea5a0", "#6d4aff", "#b45309", "#d4a72c", "#2b7a78", "#c026a6", "#b91c1c",
  "#3b82f6", "#8b5cf6", "#0891b2", "#65a30d", "#db2777", "#ea580c", "#4f46e5", "#0d9488", "#a16207",
];

const isDark = ref(false);
const mql = window.matchMedia("(prefers-color-scheme: dark)");
const sync = () => (isDark.value = mql.matches);
onMounted(() => { sync(); mql.addEventListener("change", sync); });
onUnmounted(() => mql.removeEventListener("change", sync));

const brl = (v: number) => "R$ " + Math.round(v).toLocaleString("pt-BR");

const option = computed(() => {
  const surface = isDark.value ? "#14201d" : "#ffffff";
  const tipBg = isDark.value ? "#14201d" : "#ffffff";
  const tipInk = isDark.value ? "#e8f0ed" : "#10201b";
  const split = isDark.value ? "rgba(255,255,255,.10)" : "rgba(16,32,27,.10)";
  const sorted = [...props.items].filter((i) => i.value > 0).sort((a, b) => b.value - a.value);
  const total = sorted.reduce((s, i) => s + i.value, 0) || 1;
  return {
    tooltip: {
      backgroundColor: tipBg,
      borderColor: split,
      borderWidth: 1,
      textStyle: { color: tipInk, fontSize: 12 },
      formatter: (p: any) => `<b>${p.name}</b><br/>${brl(p.value)} · ${((p.value / total) * 100).toFixed(1)}%`,
    },
    series: [
      {
        type: "treemap",
        roam: false,
        nodeClick: false,
        breadcrumb: { show: false },
        width: "100%",
        height: "100%",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        label: {
          show: true,
          color: "#fff",
          fontSize: 12,
          fontWeight: 600,
          overflow: "truncate",
          textShadowColor: "rgba(0,0,0,.35)",
          textShadowBlur: 2,
          formatter: (p: any) => `${p.name}\n${brl(p.value)}`,
        },
        itemStyle: { borderColor: surface, borderWidth: 2, gapWidth: 2, borderRadius: 4 },
        data: sorted.map((it, i) => ({
          name: it.name,
          value: it.value,
          itemStyle: { color: PALETTE[i % PALETTE.length] },
        })),
      },
    ],
  };
});

const hasData = computed(() => props.items.some((i) => i.value > 0));
</script>

<template>
  <VChart v-if="hasData" :option="option" autoresize :style="{ height }" />
  <p v-else class="tm-empty">Sem gastos para exibir.</p>
</template>

<style scoped>
.tm-empty { font-size: 12.5px; color: var(--clr-text-muted, #7c8b86); margin: 0; }
</style>
