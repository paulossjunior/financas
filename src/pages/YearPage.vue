<script setup lang="ts">
// Year page — annual summary with per-month charts, inflation, and a printable report.
import { onMounted, onUnmounted, ref, computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { LineChart } from "echarts/charts";
import { GridComponent, TooltipComponent, LegendComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import { getYearSummary, getInflation } from "@/services/tauri.service";
import type { YearSummary, InflationData } from "@/types/api.types";
import ReportOverlay from "@/components/report/ReportOverlay.vue";
import CategoryTreemap from "@/components/dashboard/CategoryTreemap.vue";
import CardForecastChart from "@/components/dashboard/CardForecastChart.vue";
import InflationCard from "@/components/dashboard/InflationCard.vue";
import InflationExplainer from "@/components/dashboard/InflationExplainer.vue";
import InflationContributions from "@/components/dashboard/InflationContributions.vue";

use([CanvasRenderer, LineChart, GridComponent, TooltipComponent, LegendComponent]);

const data = ref<YearSummary | null>(null);
const inflation = ref<InflationData | null>(null);
async function reloadInflation(): Promise<void> {
  try { inflation.value = await getInflation(); } catch { /* sem cache */ }
}
const ipcaByMonth = computed(() => {
  const m: Record<string, number> = {};
  for (const p of inflation.value?.series ?? []) m[p.month] = parseFloat(p.value) || 0;
  return m;
});
const loading = ref(true);
const error = ref<string | null>(null);
const yearFrom = ref<number | null>(null); // null = sem limite inferior
const yearTo = ref<number | null>(null);   // null = sem limite superior
const years = ref<number[]>([]);
const rangeFrom = ref<string | null>(null); // YYYY-MM
const rangeTo = ref<string | null>(null);

// Months shown in the per-month views (chart, saldo, teto), after the interval filter.
const viewMonths = computed(() => {
  const ms = data.value?.months ?? [];
  return ms.filter(
    (m) => (!rangeFrom.value || m.month >= rangeFrom.value) && (!rangeTo.value || m.month <= rangeTo.value)
  );
});

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    data.value = await getYearSummary(yearFrom.value ?? undefined, yearTo.value ?? undefined);
    // Keep the year list stable (backend returns all years regardless of filter).
    if (data.value.available_years.length) years.value = data.value.available_years;
    rangeFrom.value = null; // reset month interval on (re)load
    rangeTo.value = null;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function onYearChange(): void {
  load();
}

// Theme follows the OS preference (the app has no manual toggle).
const isDark = ref(false);
const mql = window.matchMedia("(prefers-color-scheme: dark)");
function syncTheme() { isDark.value = mql.matches; }

onMounted(async () => {
  syncTheme();
  mql.addEventListener("change", syncTheme);
  await load();
  await reloadInflation();
});
onUnmounted(() => mql.removeEventListener("change", syncTheme));

const MONTHS = ["Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez"];
const n = (s: string) => parseFloat(s) || 0;
const brl = (v: number | string) =>
  "R$ " + Math.round(typeof v === "string" ? n(v) : v).toLocaleString("pt-BR");
const brlF = (v: number | string) =>
  (typeof v === "string" ? n(v) : v).toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
function monthLabel(ym: string): string {
  const [y, m] = ym.split("-");
  return `${MONTHS[parseInt(m, 10) - 1] ?? m}/${y.slice(2)}`;
}

const hasData = computed(() => !!data.value && data.value.months.length > 0);
const fixedPerMonth = computed(() => {
  const d = data.value;
  if (!d || d.active_months === 0) return 0;
  return n(d.fixed_total) / d.active_months;
});
const savingsPct = computed(() => (data.value ? Math.round(data.value.savings_rate * 1000) / 10 : 0));
const balanceNum = computed(() => (data.value ? n(data.value.balance_total) : 0));

// Card ceiling — two simulations: all recurring income vs. salary-flagged only.
const ceilBase = ref<"all" | "salary">("all");
const ceiling = computed(() => {
  if (!data.value) return 0;
  return ceilBase.value === "all" ? n(data.value.card_ceiling) : n(data.value.card_ceiling_salary);
});
const hasCeiling = computed(() => (data.value ? n(data.value.salary_month) > 0 : false));
const ceilingScaleMax = computed(() => {
  const cards = viewMonths.value.map((m) => n(m.card));
  return Math.max(ceiling.value, ...cards, 1) * 1.08;
});
const monthsWithin = computed(() => {
  const ms = viewMonths.value;
  return { within: ms.filter((m) => n(m.card) <= ceiling.value).length, total: ms.length };
});

// Category ranking: top 8 + grouped remainder.
const ranking = computed(() => {
  const cats = (data.value?.categories ?? [])
    .map((c) => ({ name: c.name, value: n(c.net_total) }))
    .sort((a, b) => b.value - a.value);
  const top = cats.slice(0, 8);
  const rest = cats.slice(8);
  if (rest.length) {
    top.push({ name: `Outras (${rest.length})`, value: rest.reduce((a, c) => a + c.value, 0) });
  }
  return top;
});
const rankMax = computed(() => ranking.value[0]?.value || 1);

const saldoMax = computed(() =>
  Math.max(1, ...viewMonths.value.map((m) => Math.abs(n(m.balance))))
);

// ECharts option (theme-aware).
const chartOption = computed(() => {
  const d = data.value;
  if (!d) return {};
  const ms = viewMonths.value;
  const teal = isDark.value ? "#34c9a6" : "#0e7c66";
  const coral = isDark.value ? "#f0a07a" : "#cf5b34";
  const axis = isDark.value ? "#8aa39b" : "#5b6f68";
  const split = isDark.value ? "rgba(255,255,255,.08)" : "rgba(16,32,27,.08)";
  const tipBg = isDark.value ? "#14201d" : "#ffffff";
  const tipInk = isDark.value ? "#e8f0ed" : "#10201b";
  const amber = isDark.value ? "#e0a458" : "#b45309";
  const labels = ms.map((m) => monthLabel(m.month));
  const ipca = ms.map((m) => (m.month in ipcaByMonth.value ? ipcaByMonth.value[m.month] : null));
  const hasIpca = ipca.some((v) => v !== null);
  const kfmt = (v: number) => (v >= 1000 ? v / 1000 + "k" : "" + v);
  const tooltip = {
    trigger: "axis",
    backgroundColor: tipBg,
    borderColor: split,
    borderWidth: 1,
    textStyle: { color: tipInk, fontSize: 12 },
    axisPointer: { link: [{ xAxisIndex: "all" }] },
    formatter: (ps: any[]) => {
      const i = ps[0].dataIndex;
      const m = ms[i];
      const bal = n(m.balance);
      const sign = bal >= 0 ? "+" : "−";
      let s = `<b>${monthLabel(m.month)}</b><br/>Receita: ${brlF(m.income)}<br/>Despesa: ${brlF(m.expense)}<br/>`
        + `<span style="color:${axis}">Saldo: ${sign}${brlF(Math.abs(bal))}</span>`;
      if (hasIpca && ipca[i] !== null) {
        s += `<br/><span style="color:${amber}">IPCA: ${ipca[i]!.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}%</span>`;
      }
      return s;
    },
  };
  const receita = { name: "Receita", type: "line", data: ms.map((m) => n(m.income)), lineStyle: { width: 2.5 }, symbol: "circle", symbolSize: 6 };
  const despesa = { name: "Despesa", type: "line", data: ms.map((m) => n(m.expense)), lineStyle: { width: 2.5 }, symbol: "circle", symbolSize: 6, areaStyle: { color: coral, opacity: isDark.value ? 0.12 : 0.1 } };

  if (!hasIpca) {
    return {
      color: [teal, coral],
      tooltip,
      legend: { data: ["Receita", "Despesa"], top: 0, textStyle: { color: axis, fontSize: 12 }, itemWidth: 16, itemHeight: 3 },
      grid: { left: 58, right: 18, top: 34, bottom: 28 },
      xAxis: { type: "category", data: labels, axisLabel: { color: axis, fontSize: 11 }, axisLine: { lineStyle: { color: split } }, axisTick: { show: false } },
      yAxis: { type: "value", axisLabel: { color: axis, fontSize: 11, formatter: kfmt }, splitLine: { lineStyle: { color: split } } },
      series: [receita, despesa],
    };
  }
  // Two stacked panels sharing the month axis: R$ on top, IPCA % below (no dual-axis on one plot).
  return {
    color: [teal, coral, amber],
    tooltip,
    legend: { data: ["Receita", "Despesa", "IPCA %"], top: 0, textStyle: { color: axis, fontSize: 12 }, itemWidth: 16, itemHeight: 3 },
    grid: [
      { left: 58, right: 18, top: 34, height: "52%" },
      { left: 58, right: 18, top: "74%", height: "16%" },
    ],
    xAxis: [
      { type: "category", data: labels, gridIndex: 0, axisLabel: { show: false }, axisLine: { lineStyle: { color: split } }, axisTick: { show: false } },
      { type: "category", data: labels, gridIndex: 1, axisLabel: { color: axis, fontSize: 11 }, axisLine: { lineStyle: { color: split } }, axisTick: { show: false } },
    ],
    yAxis: [
      { type: "value", gridIndex: 0, axisLabel: { color: axis, fontSize: 11, formatter: kfmt }, splitLine: { lineStyle: { color: split } } },
      { type: "value", gridIndex: 1, axisLabel: { color: axis, fontSize: 10, formatter: (v: number) => v + "%" }, splitLine: { show: false } },
    ],
    series: [
      { ...receita, xAxisIndex: 0, yAxisIndex: 0 },
      { ...despesa, xAxisIndex: 0, yAxisIndex: 0 },
      { name: "IPCA %", type: "line", xAxisIndex: 1, yAxisIndex: 1, data: ipca, connectNulls: true, lineStyle: { width: 2, color: amber }, itemStyle: { color: amber }, symbol: "circle", symbolSize: 4 },
    ],
  };
});

// ── Report overlay (print / PDF) ── respects the active year + month filter.
const reportOpen = ref(false);
const genDate = new Date().toLocaleDateString("pt-BR");
const pctOf = (v: number, max: number) => Math.max((v / max) * 100, 2);
const filterLabel = computed(() => {
  const yl = yearFrom.value && yearTo.value
    ? (yearFrom.value === yearTo.value ? `${yearFrom.value}` : `${yearFrom.value} → ${yearTo.value}`)
    : yearFrom.value ? `desde ${yearFrom.value}` : yearTo.value ? `até ${yearTo.value}` : "todos os anos";
  const ms = viewMonths.value;
  const ml = ms.length ? `${monthLabel(ms[0].month)}–${monthLabel(ms[ms.length - 1].month)}` : "—";
  return `📅 ${yl} · ${ml}`;
});
const reportTitle = computed(() => `Relatório do período · ${filterLabel.value}`);
const activeMonths = computed(() => viewMonths.value.length);
const periodIncome = computed(() => viewMonths.value.reduce((a, m) => a + n(m.income), 0));
const periodExpense = computed(() => viewMonths.value.reduce((a, m) => a + n(m.expense), 0));
const periodCard = computed(() => viewMonths.value.reduce((a, m) => a + n(m.card), 0));
const periodFixed = computed(() => viewMonths.value.reduce((a, m) => a + n(m.fixed), 0));
const periodVariable = computed(() => viewMonths.value.reduce((a, m) => a + n(m.variable), 0));
const periodPayroll = computed(() => viewMonths.value.reduce((a, m) => a + n(m.payroll), 0));
const periodBalance = computed(() => periodIncome.value - periodExpense.value);
const pctExp = (v: number) => (periodExpense.value > 0 ? (v / periodExpense.value) * 100 : 0);

// Inline SVG line chart (income vs expense) — reliable for print, independent of ECharts canvas.
const svgChart = computed(() => {
  const ms = viewMonths.value;
  const inc = ms.map((m) => n(m.income));
  const exp = ms.map((m) => n(m.expense));
  const maxV = Math.max(1, ...inc, ...exp);
  const padL = 44, padR = 16, padT = 16, padB = 28, W = 680, H = 260;
  const plotW = W - padL - padR, plotH = H - padT - padB;
  const xAt = (i: number) => (ms.length <= 1 ? padL + plotW / 2 : padL + (i * plotW) / (ms.length - 1));
  const yAt = (v: number) => padT + (1 - v / maxV) * plotH;
  const pts = (arr: number[]) => arr.map((v, i) => `${xAt(i).toFixed(1)},${yAt(v).toFixed(1)}`).join(" ");
  const dots = (arr: number[]) => arr.map((v, i) => ({ cx: +xAt(i).toFixed(1), cy: +yAt(v).toFixed(1) }));
  const grid = [0, 0.25, 0.5, 0.75, 1].map((f) => ({
    y: +yAt(maxV * f).toFixed(1),
    label: Math.round((maxV * f) / 1000) + "k",
  }));
  const labels = ms.map((m, i) => ({ x: +xAt(i).toFixed(1), t: MONTHS[parseInt(m.month.split("-")[1], 10) - 1] }));
  return { W, H, incPts: pts(inc), expPts: pts(exp), incDots: dots(inc), expDots: dots(exp), grid, labels };
});

// ── Matriz-seletor: categoria × ano (o próprio seletor) ──
const CAT_PALETTE = ["#0e7c66", "#0ea5a0", "#6d4aff", "#b45309", "#d4a72c", "#2b7a78", "#c026a6", "#b91c1c"];
const selectedCats = ref<Set<string>>(new Set());
const isSel = (name: string) => selectedCats.value.has(name);
function toggleCat(name: string): void {
  const s = new Set(selectedCats.value);
  s.has(name) ? s.delete(name) : s.add(name);
  selectedCats.value = s;
}
function selectAllCats(): void { selectedCats.value = new Set(matrixRows.value.map((r) => r.name)); }
function clearCats(): void { selectedCats.value = new Set(); }

const yearsInRange = computed(() => {
  const s = new Set<number>();
  for (const m of viewMonths.value) s.add(parseInt(m.month.split("-")[0], 10));
  return [...s].sort((a, b) => a - b);
});
// Aggregate expense per category: period total + per-year, over the filtered months.
const catAgg = computed(() => {
  const total = new Map<string, number>();
  const byYear = new Map<string, Map<number, number>>();
  for (const m of viewMonths.value) {
    const y = parseInt(m.month.split("-")[0], 10);
    for (const c of m.categories) {
      const v = n(c.net_total);
      total.set(c.name, (total.get(c.name) ?? 0) + v);
      let ym = byYear.get(c.name);
      if (!ym) { ym = new Map(); byYear.set(c.name, ym); }
      ym.set(y, (ym.get(y) ?? 0) + v);
    }
  }
  return { total, byYear };
});
const matrixRows = computed(() =>
  [...catAgg.value.total.entries()]
    .map(([name, tot]) => ({ name, total: tot, byYear: catAgg.value.byYear.get(name) ?? new Map<number, number>() }))
    .sort((a, b) => b.total - a.total)
);
const maxCell = computed(() => {
  let mx = 1;
  for (const r of matrixRows.value) for (const y of yearsInRange.value) mx = Math.max(mx, r.byYear.get(y) ?? 0);
  return mx;
});
const cellPct = (v: number) => Math.round((v / maxCell.value) * 100);
// Footer totals: selected categories (or all, when nothing is selected).
const totalRowSet = computed(() => (selectedCats.value.size ? matrixRows.value.filter((r) => isSel(r.name)) : matrixRows.value));
const totalsByYear = computed(() => {
  const map = new Map<number, number>();
  for (const r of totalRowSet.value) for (const y of yearsInRange.value) map.set(y, (map.get(y) ?? 0) + (r.byYear.get(y) ?? 0));
  return map;
});
const grandTotalSel = computed(() => [...totalsByYear.value.values()].reduce((a, b) => a + b, 0));

// Treemap items: selection-aware for the screen, all categories for the report.
const treemapItemsAll = computed(() => matrixRows.value.map((r) => ({ name: r.name, value: r.total })));
const treemapItemsYear = computed(() =>
  (selectedCats.value.size ? matrixRows.value.filter((r) => isSel(r.name)) : matrixRows.value).map((r) => ({ name: r.name, value: r.total }))
);

// Multi-line chart driven by the selection: one line per category + a bold Total line.
const hasSelection = computed(() => selectedCats.value.size > 0);
const selNames = computed(() => matrixRows.value.filter((r) => isSel(r.name)).map((r) => r.name));
const monthCatValue = (m: YearSummary["months"][number], name: string) => {
  const c = m.categories.find((x) => x.name === name);
  return c ? n(c.net_total) : 0;
};
const selKpis = computed(() => {
  if (!hasSelection.value) return null;
  const ms = viewMonths.value;
  const perMonth = ms.map((m) => ({ month: m.month, v: selNames.value.reduce((a, name) => a + monthCatValue(m, name), 0) }));
  const total = perMonth.reduce((a, p) => a + p.v, 0);
  const biggest = perMonth.reduce((mx, p) => (p.v > mx.v ? p : mx), { month: "", v: 0 });
  return { total, avg: total / Math.max(1, ms.length), biggestMonth: biggest.month ? monthLabel(biggest.month) : "—", biggestVal: biggest.v };
});
const selChartOption = computed(() => {
  if (!hasSelection.value) return {};
  const ms = viewMonths.value;
  const axis = isDark.value ? "#8aa39b" : "#5b6f68";
  const split = isDark.value ? "rgba(255,255,255,.08)" : "rgba(16,32,27,.08)";
  const tipBg = isDark.value ? "#14201d" : "#ffffff";
  const tipInk = isDark.value ? "#e8f0ed" : "#10201b";
  const totalColor = isDark.value ? "#e8efec" : "#16211e";
  const labels = ms.map((m) => monthLabel(m.month));
  const catSeries = selNames.value.map((name, i) => ({
    name,
    type: "line",
    smooth: false,
    symbol: "circle",
    symbolSize: 5,
    lineStyle: { width: 2 },
    itemStyle: { color: CAT_PALETTE[i % CAT_PALETTE.length] },
    data: ms.map((m) => monthCatValue(m, name)),
  }));
  const total = {
    name: "Total",
    type: "line",
    smooth: false,
    symbol: "circle",
    symbolSize: 6,
    z: 5,
    lineStyle: { width: 3.2 },
    itemStyle: { color: totalColor },
    data: ms.map((m) => selNames.value.reduce((a, name) => a + monthCatValue(m, name), 0)),
  };
  return {
    color: [...selNames.value.map((_, i) => CAT_PALETTE[i % CAT_PALETTE.length]), totalColor],
    tooltip: { trigger: "axis", backgroundColor: tipBg, borderColor: split, borderWidth: 1, textStyle: { color: tipInk, fontSize: 12 },
      valueFormatter: (v: number) => brlF(v) },
    legend: { data: [...selNames.value, "Total"], top: 0, textStyle: { color: axis, fontSize: 12 }, itemWidth: 16, itemHeight: 3, type: "scroll" },
    grid: { left: 58, right: 18, top: 34, bottom: 28 },
    xAxis: { type: "category", data: labels, axisLabel: { color: axis, fontSize: 11 }, axisLine: { lineStyle: { color: split } }, axisTick: { show: false } },
    yAxis: { type: "value", axisLabel: { color: axis, fontSize: 11, formatter: (v: number) => (v >= 1000 ? v / 1000 + "k" : "" + v) }, splitLine: { lineStyle: { color: split } } },
    series: [...catSeries, total],
  };
});
</script>

<template>
  <div class="page">
    <header class="top">
      <div class="titles">
        <p class="eyebrow">Visão anual</p>
        <h1>Receita vs. gastos do ano</h1>
        <p class="sub" v-if="data">
          {{ data.months.length ? `${monthLabel(data.months[0].month)} – ${monthLabel(data.months[data.months.length-1].month)}` : "sem lançamentos" }}
          · {{ data.tx_count }} lançamentos no cartão
        </p>
      </div>
      <div class="yearfilter" v-if="years.length">
        <span>Ano</span>
        <select v-model="yearFrom" @change="onYearChange" aria-label="Ano inicial">
          <option :value="null">início</option>
          <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
        </select>
        <span class="dash">—</span>
        <select v-model="yearTo" @change="onYearChange" aria-label="Ano final">
          <option :value="null">fim</option>
          <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
        </select>
        <button v-if="hasData" class="reportbtn" @click="reportOpen = true">📄 Relatório</button>
      </div>
    </header>

    <div v-if="loading" class="state">Carregando…</div>
    <div v-else-if="error" class="state err">⚠ {{ error }}</div>
    <div v-else-if="!hasData" class="state">Importe faturas ou cadastre lançamentos para ver a visão anual.</div>

    <template v-else-if="data">
      <!-- KPIs -->
      <div class="kpis">
        <div class="kpi">
          <span class="lbl">Receita</span>
          <span class="val pos">{{ brl(data.income_total) }}</span>
          <span class="sub2" v-if="n(data.income_total) > 0">no período</span>
          <span class="sub2 warn" v-else>cadastre em Receitas &amp; Fixos</span>
        </div>
        <div class="kpi">
          <span class="lbl">Despesa total</span>
          <span class="val exp">{{ brl(data.expense_total) }}</span>
          <span class="sub2">cartão {{ brl(data.card_total) }} + fixos {{ brl(data.fixed_total) }}<template v-if="n(data.variable_total) > 0"> + avulsos {{ brl(data.variable_total) }}</template><template v-if="n(data.payroll_total) > 0"> + descontos {{ brl(data.payroll_total) }}</template></span>
        </div>
        <div class="kpi">
          <span class="lbl">Saldo do período</span>
          <span class="val" :class="balanceNum >= 0 ? 'pos' : 'neg'">
            {{ balanceNum >= 0 ? "+ " : "− " }}{{ brl(Math.abs(balanceNum)) }}
          </span>
          <span class="sub2" v-if="n(data.income_total) > 0">poupança {{ savingsPct }}%</span>
          <span class="sub2" v-else>sem receita cadastrada</span>
        </div>
        <div class="kpi">
          <span class="lbl">Despesa média</span>
          <span class="val">{{ brl(data.avg_expense) }}</span>
          <span class="sub2">por mês</span>
        </div>
        <div class="kpi">
          <span class="lbl">Maior mês</span>
          <span class="val">{{ brl(data.biggest_month_value) }}</span>
          <span class="sub2">{{ data.biggest_month ? monthLabel(data.biggest_month) : "—" }}</span>
        </div>
        <div class="kpi">
          <span class="lbl">Fixos / mês</span>
          <span class="val">{{ brl(fixedPerMonth) }}</span>
          <span class="sub2">recorrentes</span>
        </div>
        <div class="kpi">
          <span class="lbl">Teto do cartão</span>
          <span class="val" v-if="hasCeiling">{{ brl(ceiling) }}</span>
          <span class="val" style="color:var(--clr-text-muted)" v-else>—</span>
          <span class="sub2" v-if="hasCeiling">base: {{ ceilBase === 'all' ? 'renda recorrente' : 'só salário' }}</span>
          <span class="sub2 warn" v-else>cadastre o salário (receita recorrente)</span>
        </div>
      </div>

      <!-- Line chart -->
      <div class="card">
        <h2>Receita vs. despesa por mês
          <span class="range" v-if="data.months.length > 1">
            <select v-model="rangeFrom" aria-label="De">
              <option :value="null">início</option>
              <option v-for="m in data.months" :key="m.month" :value="m.month">{{ monthLabel(m.month) }}</option>
            </select>
            <span class="dash">—</span>
            <select v-model="rangeTo" aria-label="Até">
              <option :value="null">fim</option>
              <option v-for="m in data.months" :key="m.month" :value="m.month">{{ monthLabel(m.month) }}</option>
            </select>
          </span>
        </h2>
        <p class="hint">Despesa = cartão (por data da compra) + gastos fixos. Use o intervalo para focar em um período.</p>
        <VChart v-if="viewMonths.length" :option="chartOption" autoresize style="height: 340px" />
        <p v-else class="hint">Intervalo vazio — ajuste De/Até.</p>
      </div>

      <!-- Saldo mensal -->
      <div class="card">
        <h2>Saldo mensal <span class="hint inline">receita − despesa</span></h2>
        <div class="saldo">
          <div class="scol" v-for="m in viewMonths" :key="m.month">
            <div class="sbarwrap">
              <div v-if="n(m.balance) >= 0" class="sbar pos" :style="{ height: (Math.abs(n(m.balance)) / saldoMax * 46) + 'px' }"></div>
              <div class="szero"></div>
              <div v-if="n(m.balance) < 0" class="sbar neg" :style="{ height: (Math.abs(n(m.balance)) / saldoMax * 46) + 'px' }"></div>
            </div>
            <span class="sval" :class="n(m.balance) >= 0 ? 'pos' : 'neg'">
              {{ n(m.balance) >= 0 ? "+" : "−" }}{{ Math.round(Math.abs(n(m.balance)) / 100) / 10 }}k
            </span>
            <span class="smth">{{ monthLabel(m.month).split("/")[0] }}</span>
          </div>
        </div>
      </div>

      <!-- Teto do cartão -->
      <div class="card" v-if="hasCeiling">
        <h2>Teto do cartão <span class="hint inline">duas simulações — clique para comparar</span></h2>
        <div class="ceil-sims">
          <button type="button" class="sim" :class="{ on: ceilBase === 'all' }" @click="ceilBase = 'all'">
            <span class="sim-l">Renda recorrente</span>
            <span class="sim-v">{{ brl(data.card_ceiling) }}</span>
            <span class="sim-s">{{ brl(data.salary_month) }} − fixos {{ brl(data.fixed_month) }}</span>
          </button>
          <button type="button" class="sim" :class="{ on: ceilBase === 'salary' }" @click="ceilBase = 'salary'">
            <span class="sim-l">Só salário</span>
            <span class="sim-v">{{ brl(data.card_ceiling_salary) }}</span>
            <span class="sim-s">{{ brl(data.salary_only) }} − fixos {{ brl(data.fixed_month) }}</span>
          </button>
        </div>
        <p class="hint">
          {{ monthsWithin.within }}/{{ monthsWithin.total }} meses dentro do teto
          ({{ ceilBase === 'all' ? 'renda recorrente' : 'só salário' }}). Barra vermelha = estourou.
        </p>
        <div class="ceil">
          <div class="crow" v-for="m in viewMonths" :key="m.month">
            <span class="cmth">{{ monthLabel(m.month) }}</span>
            <div class="ctrack">
              <div class="cfill" :class="n(m.card) > ceiling ? 'over' : 'ok'" :style="{ width: Math.min(n(m.card) / ceilingScaleMax * 100, 100) + '%' }"></div>
              <div class="cmark" :style="{ left: (ceiling / ceilingScaleMax * 100) + '%' }" title="Teto"></div>
            </div>
            <span class="camt" :class="n(m.card) > ceiling ? 'over' : 'ok'">{{ brl(m.card) }}</span>
          </div>
        </div>
      </div>

      <!-- Inflação (IPCA + pessoal) -->
      <div class="card">
        <InflationCard @updated="reloadInflation" />
      </div>

      <div class="card" v-if="inflation?.available">
        <InflationExplainer
          :data="inflation"
          :monthly-expense="periodExpense / Math.max(1, activeMonths)"
          :monthly-income="periodIncome / Math.max(1, activeMonths)"
        />
      </div>

      <div class="card" v-if="inflation?.available">
        <InflationContributions />
      </div>

      <!-- Previsão do cartão (parcelamentos) -->
      <div class="card">
        <h2>Previsão do cartão <span class="hint inline">o que os parcelamentos já comprometem</span></h2>
        <p class="hint">Quanto do cartão já está comprometido em cada mês à frente pelas compras parceladas. Passe o mouse para ver as parcelas.</p>
        <CardForecastChart :points="data.card_forecast" table />
      </div>

      <div class="row2">
        <!-- ranking -->
        <div class="card">
          <h2>Para onde foi o dinheiro</h2>
          <p class="hint">Top categorias no período (inclui fixos)</p>
          <div class="bars">
            <div class="bar" v-for="(c, i) in ranking" :key="c.name" :class="{ lead: i === 0 }">
              <div class="brow"><span class="nm">{{ c.name }}</span><span class="amt">{{ brl(c.value) }}</span></div>
              <div class="track"><div class="fill" :class="{ other: c.name.startsWith('Outras') }" :style="{ width: (c.value / rankMax * 100) + '%' }"></div></div>
            </div>
          </div>
        </div>

        <!-- fixo vs variavel -->
        <div class="card">
          <h2>Fixo vs. variável</h2>
          <p class="hint">Total do período</p>
          <div class="segbar">
            <div class="s1" :style="{ width: (n(data.expense_total) ? n(data.fixed_total) / n(data.expense_total) * 100 : 0) + '%' }"></div>
            <div class="s2" :style="{ width: (n(data.expense_total) ? n(data.card_total) / n(data.expense_total) * 100 : 0) + '%' }"></div>
            <div v-if="n(data.variable_total) > 0" class="s4" :style="{ width: (n(data.expense_total) ? n(data.variable_total) / n(data.expense_total) * 100 : 0) + '%' }"></div>
            <div class="s3" :style="{ width: (n(data.expense_total) ? n(data.payroll_total) / n(data.expense_total) * 100 : 0) + '%' }"></div>
          </div>
          <div class="leg">
            <span><i class="dotc fix"></i> Fixo · {{ brl(data.fixed_total) }}</span>
            <span><i class="dotc var"></i> Cartão · {{ brl(data.card_total) }}</span>
            <span v-if="n(data.variable_total) > 0"><i class="dotc avul"></i> Avulsos · {{ brl(data.variable_total) }}</span>
            <span v-if="n(data.payroll_total) > 0"><i class="dotc ded"></i> Descontos · {{ brl(data.payroll_total) }}</span>
          </div>
        </div>
      </div>

      <!-- Matriz-seletor: categoria × ano (o próprio seletor alimenta o gráfico) -->
      <div class="card" v-if="matrixRows.length">
        <div class="mxbar">
          <div>
            <h2>Categorias × ano</h2>
            <p class="hint" style="margin:0">Clique na linha para marcar/desmarcar. O gráfico abaixo segue a seleção.</p>
          </div>
          <div class="mxtools">
            <button class="tbtn" @click="selectAllCats">Marcar todas</button>
            <button class="tbtn" @click="clearCats">Limpar</button>
            <span class="yrs">Ano
              <select v-model="yearFrom" @change="onYearChange" aria-label="Ano inicial">
                <option :value="null">início</option>
                <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
              </select>
              <span class="dash">—</span>
              <select v-model="yearTo" @change="onYearChange" aria-label="Ano final">
                <option :value="null">fim</option>
                <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
              </select>
            </span>
          </div>
        </div>
        <div class="mxwrap">
          <table class="mx">
            <thead>
              <tr>
                <th>Categoria</th>
                <th v-for="y in yearsInRange" :key="y">{{ y }}</th>
                <th>Total</th>
                <th>Média</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in matrixRows" :key="r.name" :class="{ sel: isSel(r.name) }" @click="toggleCat(r.name)">
                <td>
                  <span class="catname">
                    <span class="ck" :class="{ on: isSel(r.name) }">{{ isSel(r.name) ? "✓" : "" }}</span>
                    {{ r.name }}
                  </span>
                </td>
                <td v-for="y in yearsInRange" :key="y">
                  <span
                    v-if="(r.byYear.get(y) ?? 0) !== 0"
                    class="cell"
                    :style="{ backgroundColor: `color-mix(in srgb, var(--clr-accent) ${cellPct(r.byYear.get(y) ?? 0)}%, transparent)`, color: cellPct(r.byYear.get(y) ?? 0) > 52 ? '#fff' : undefined }"
                  >{{ brl(r.byYear.get(y) ?? 0) }}</span>
                  <span v-else class="cell muted">—</span>
                </td>
                <td><b>{{ brl(r.total) }}</b></td>
                <td>{{ brl(r.total / Math.max(1, yearsInRange.length)) }}</td>
              </tr>
              <tr class="tot">
                <td>{{ selectedCats.size ? `Total (${selectedCats.size} selec.)` : "Total (todas)" }}</td>
                <td v-for="y in yearsInRange" :key="y">{{ brl(totalsByYear.get(y) ?? 0) }}</td>
                <td>{{ brl(grandTotalSel) }}</td>
                <td>{{ brl(grandTotalSel / Math.max(1, yearsInRange.length)) }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="heat-legend"><span>menor</span><span class="heat-bar"></span><span>maior</span> · intensidade por valor</div>
      </div>

      <!-- Gráfico multi-linha da seleção -->
      <div class="card" v-if="matrixRows.length">
        <h2>Evolução por categoria <span class="hint inline">uma linha por categoria + Total</span></h2>
        <template v-if="hasSelection && viewMonths.length">
          <div class="selkpis" v-if="selKpis">
            <div class="skpi"><span class="l">Total (seleção)</span><span class="v">{{ brl(selKpis.total) }}</span></div>
            <div class="skpi"><span class="l">Média / mês</span><span class="v">{{ brl(selKpis.avg) }}</span></div>
            <div class="skpi"><span class="l">Maior mês</span><span class="v">{{ selKpis.biggestMonth }}</span><span class="s">{{ brl(selKpis.biggestVal) }}</span></div>
          </div>
          <VChart :option="selChartOption" autoresize style="height: 320px" />
        </template>
        <p v-else class="hint">Marque categorias na matriz acima para ver a evolução mês a mês (uma linha por categoria + uma linha de Total).</p>
      </div>

      <!-- Treemap: mapa de gastos do período -->
      <div class="card" v-if="treemapItemsYear.length">
        <h2>Mapa de gastos <span class="hint inline">{{ selectedCats.size ? "categorias selecionadas" : "todas as categorias" }}</span></h2>
        <p class="hint">Área proporcional ao valor. Respeita o filtro de ano e a seleção da matriz.</p>
        <CategoryTreemap :items="treemapItemsYear" height="360px" />
      </div>
    </template>

    <!-- ── Report (print / PDF) — respects year + month filter ── -->
    <ReportOverlay v-if="reportOpen && data" :title="reportTitle" @close="reportOpen = false">
      <div class="sheet">
        <div class="sheet-head">
          <div class="logo">₣</div>
          <div>
            <div class="t">Relatório do período</div>
            <div class="s">Receitas × despesas, tendência e ranking · {{ activeMonths }} {{ activeMonths === 1 ? "mês" : "meses" }}</div>
          </div>
          <div class="right"><span class="filterchip">{{ filterLabel }}</span><br>gerado em {{ genDate }}</div>
        </div>
        <div class="sheet-body">

          <div class="kpis">
            <div class="kpi"><div class="l">Meses ativos</div><div class="v">{{ activeMonths }}</div><div class="sub">no filtro</div></div>
            <div class="kpi"><div class="l">Receita total</div><div class="v pos">{{ brl(periodIncome) }}</div><div class="sub">média {{ brl(periodIncome / Math.max(1, activeMonths)) }}/mês</div></div>
            <div class="kpi"><div class="l">Despesa total</div><div class="v">{{ brl(periodExpense) }}</div><div class="sub">média {{ brl(periodExpense / Math.max(1, activeMonths)) }}/mês</div></div>
            <div class="kpi"><div class="l">Saldo do período</div><div class="v" :class="periodBalance >= 0 ? 'pos' : 'neg'">{{ periodBalance >= 0 ? "" : "− " }}{{ brl(Math.abs(periodBalance)) }}</div><div class="sub">{{ periodBalance >= 0 ? "sobra" : "déficit" }}</div></div>
          </div>

          <div>
            <h3>Composição da despesa</h3>
            <p class="cap">Cartão, fixos, avulsos e descontos da folha somados no período.</p>
            <div class="compbar">
              <div v-if="periodCard > 0" class="seg card" :style="{ flexGrow: periodCard }"><span v-if="pctExp(periodCard) > 10">Cartão {{ pctExp(periodCard).toFixed(1) }}%</span></div>
              <div v-if="periodFixed > 0" class="seg fix" :style="{ flexGrow: periodFixed }"><span v-if="pctExp(periodFixed) > 10">Fixos {{ pctExp(periodFixed).toFixed(1) }}%</span></div>
              <div v-if="periodVariable > 0" class="seg avul" :style="{ flexGrow: periodVariable }"><span v-if="pctExp(periodVariable) > 10">Avulsos {{ pctExp(periodVariable).toFixed(1) }}%</span></div>
              <div v-if="periodPayroll > 0" class="seg ded" :style="{ flexGrow: periodPayroll }"><span v-if="pctExp(periodPayroll) > 10">Descontos {{ pctExp(periodPayroll).toFixed(1) }}%</span></div>
            </div>
            <div class="legend">
              <span v-if="periodCard > 0"><i class="dot card"></i> Cartão — {{ brl(periodCard) }}</span>
              <span v-if="periodFixed > 0"><i class="dot fix"></i> Fixos — {{ brl(periodFixed) }}</span>
              <span v-if="periodVariable > 0"><i class="dot avul"></i> Avulsos — {{ brl(periodVariable) }}</span>
              <span v-if="periodPayroll > 0"><i class="dot ded"></i> Descontos — {{ brl(periodPayroll) }}</span>
            </div>
          </div>

          <div>
            <h3>Receita × despesa, mês a mês</h3>
            <p class="cap">Linha do tempo do período filtrado.</p>
            <div class="chartwrap">
              <svg :viewBox="`0 0 ${svgChart.W} ${svgChart.H}`" role="img" aria-label="Receita e despesa por mês">
                <line v-for="(g, i) in svgChart.grid" :key="'g' + i" class="gl" x1="44" :y1="g.y" x2="664" :y2="g.y" />
                <text v-for="(g, i) in svgChart.grid" :key="'gt' + i" class="gt" x="38" :y="g.y + 3" text-anchor="end">{{ g.label }}</text>
                <polyline class="line-inc" fill="none" stroke-width="2.5" stroke-linejoin="round" :points="svgChart.incPts" />
                <polyline class="line-exp" fill="none" stroke-width="2.5" stroke-linejoin="round" :points="svgChart.expPts" />
                <circle v-for="(dt, i) in svgChart.incDots" :key="'i' + i" class="dot-inc" :cx="dt.cx" :cy="dt.cy" r="3.5" />
                <circle v-for="(dt, i) in svgChart.expDots" :key="'e' + i" class="dot-exp" :cx="dt.cx" :cy="dt.cy" r="3.5" />
                <text v-for="(l, i) in svgChart.labels" :key="'l' + i" class="ml" :x="l.x" y="250" text-anchor="middle">{{ l.t }}</text>
              </svg>
            </div>
            <div class="legend">
              <span><i class="dot inc"></i> Receita</span>
              <span><i class="dot exp"></i> Despesa</span>
            </div>
          </div>

          <div>
            <h3>Mês a mês</h3>
            <div class="tblwrap">
              <table>
                <thead><tr><th>Mês</th><th>Cartão</th><th>Fixos</th><th>Avulsos</th><th>Descontos</th><th>Receita</th><th>Saldo</th></tr></thead>
                <tbody>
                  <tr v-for="m in viewMonths" :key="m.month">
                    <td>{{ monthLabel(m.month) }}</td>
                    <td>{{ brl(m.card) }}</td>
                    <td>{{ brl(m.fixed) }}</td>
                    <td>{{ n(m.variable) > 0 ? brl(m.variable) : "—" }}</td>
                    <td>{{ n(m.payroll) > 0 ? brl(m.payroll) : "—" }}</td>
                    <td>{{ brl(m.income) }}</td>
                    <td :class="n(m.balance) >= 0 ? 'ok' : 'over'">{{ n(m.balance) >= 0 ? "" : "− " }}{{ brl(Math.abs(n(m.balance))) }}</td>
                  </tr>
                  <tr class="tot">
                    <td>Total</td><td>{{ brl(periodCard) }}</td><td>{{ brl(periodFixed) }}</td>
                    <td>{{ periodVariable > 0 ? brl(periodVariable) : "—" }}</td><td>{{ brl(periodPayroll) }}</td>
                    <td>{{ brl(periodIncome) }}</td>
                    <td :class="periodBalance >= 0 ? 'ok' : 'over'">{{ periodBalance >= 0 ? "" : "− " }}{{ brl(Math.abs(periodBalance)) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div v-if="hasCeiling">
            <h3>Teto do cartão</h3>
            <p class="cap">Do contracheque mais recente (renda − contas fixas mensais).</p>
            <div class="ceil">
              <div class="sim">
                <div class="h">Com renda recorrente</div>
                <div class="v">{{ brl(data.card_ceiling) }}</div>
                <div class="f">{{ brl(data.salary_month) }} − fixos {{ brl(data.fixed_month) }}</div>
              </div>
              <div class="sim">
                <div class="h">Só salário permanente</div>
                <div class="v">{{ brl(data.card_ceiling_salary) }}</div>
                <div class="f">{{ brl(data.salary_only) }} − fixos {{ brl(data.fixed_month) }}</div>
              </div>
            </div>
          </div>

          <div v-if="treemapItemsAll.length">
            <h3>Mapa de gastos</h3>
            <p class="cap">Área proporcional ao valor da categoria no período.</p>
            <CategoryTreemap :items="treemapItemsAll" height="300px" />
          </div>

          <div v-if="ranking.length">
            <h3>Top categorias no período</h3>
            <p class="cap">Maiores gastos do cartão no filtro.</p>
            <div class="rank">
              <div v-for="c in ranking" :key="c.name" class="rk">
                <span class="n">{{ c.name }}</span>
                <span class="bar" :style="{ width: pctOf(c.value, rankMax) + '%' }"></span>
                <b>{{ brl(c.value) }}</b>
              </div>
            </div>
          </div>

          <div class="insight" :class="{ warn: periodBalance < 0 }">
            <b>Leitura do período:</b>
            <template v-if="periodBalance < 0"> despesas superaram receitas em {{ brl(Math.abs(periodBalance)) }}.</template>
            <template v-else> sobra de {{ brl(periodBalance) }} ({{ savingsPct }}% da receita).</template>
            Cartão médio de {{ brl(periodCard / Math.max(1, activeMonths)) }}/mês; maior mês foi {{ monthLabel(data.biggest_month) }}.
          </div>

        </div>
      </div>
    </ReportOverlay>
  </div>
</template>

<style scoped>
.page { padding: 1.75rem 2rem 4rem; max-width: 1320px; margin: 0 auto; color: var(--clr-text-primary); font-variant-numeric: tabular-nums; }
.top { margin-bottom: 1.25rem; display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
.yearfilter { display: flex; align-items: center; gap: .5rem; font-size: 12px; font-weight: 600; color: var(--clr-text-secondary); }
.yearfilter .dash { color: var(--clr-text-muted); }
.yearfilter select {
  font-family: inherit; font-size: 13px; font-weight: 600; padding: 6px 10px;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md);
  background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer; outline: none;
}
.yearfilter select:focus { border-color: var(--clr-accent); }
.reportbtn {
  font-family: inherit; font-size: 13px; font-weight: 700; padding: 6px 12px; margin-left: .4rem;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md);
  background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer;
}
.reportbtn:hover { border-color: var(--clr-accent); color: var(--clr-accent); }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--clr-accent); font-weight: 700; margin: 0 0 6px; }
h1 { font-size: 26px; font-weight: 800; letter-spacing: -.02em; margin: 0 0 6px; }
.sub { color: var(--clr-text-secondary); font-size: 13px; margin: 0; }

.state { padding: 2rem 0; color: var(--clr-text-secondary); font-size: 14px; }
.state.err { color: var(--clr-negative); }

.kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(172px, 1fr)); gap: .7rem; margin-bottom: 1rem; }
.kpi { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: var(--radius-lg); padding: .9rem 1rem; box-shadow: var(--shadow-sm); display: flex; flex-direction: column; gap: .15rem; }
.kpi .lbl { font-size: 10.5px; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: var(--clr-text-muted); }
.kpi .val { font-size: 1.4rem; font-weight: 780; letter-spacing: -.02em; }
.kpi .val.pos { color: var(--clr-positive); } .kpi .val.neg { color: var(--clr-negative); }
.kpi .val.exp { color: var(--clr-amber); }
.kpi .sub2 { font-size: 11px; color: var(--clr-text-muted); }
.kpi .sub2.warn { color: var(--clr-amber); }

.card { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: var(--radius-lg); box-shadow: var(--shadow-sm); padding: 1.2rem 1.3rem; margin-bottom: 1rem; }
.card h2 { font-size: .95rem; font-weight: 700; letter-spacing: -.01em; margin: 0 0 .1rem; display: flex; align-items: center; justify-content: space-between; gap: .6rem; flex-wrap: wrap; }
.range { display: flex; align-items: center; gap: .4rem; }
.range .dash { color: var(--clr-text-muted); }
.range select {
  font-family: inherit; font-size: 12px; font-weight: 600; padding: 4px 8px;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md);
  background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer; outline: none;
}
.range select:focus { border-color: var(--clr-accent); }
.card .hint { font-size: .78rem; color: var(--clr-text-muted); margin: 0 0 .9rem; }
.card .hint.inline { margin: 0 0 0 .5rem; font-weight: 500; }

.row2 { display: grid; grid-template-columns: 1.5fr 1fr; gap: 1rem; }

/* saldo bars */
.saldo { display: flex; gap: 4px; align-items: flex-end; padding-top: .5rem; }
.scol { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 4px; }
.sbarwrap { height: 104px; display: flex; flex-direction: column; align-items: center; justify-content: center; width: 100%; }
.sbar { width: 60%; max-width: 34px; border-radius: 4px; }
.sbar.pos { background: var(--clr-positive); }
.sbar.neg { background: var(--clr-negative); }
.szero { height: 1px; width: 100%; background: var(--clr-stroke); }
.sval { font-size: 10.5px; font-weight: 700; }
.sval.pos { color: var(--clr-positive); } .sval.neg { color: var(--clr-negative); }
.smth { font-size: 10.5px; color: var(--clr-text-muted); }

/* card ceiling */
.ceil-sims { display: grid; grid-template-columns: 1fr 1fr; gap: .7rem; margin-bottom: 1rem; }
.sim {
  text-align: left; font-family: inherit; cursor: pointer;
  display: flex; flex-direction: column; gap: .15rem;
  padding: .8rem .9rem; border-radius: 12px;
  border: 1.5px solid var(--clr-stroke); background: var(--clr-surface); color: var(--clr-text-primary);
  transition: border-color .1s, background .1s;
}
.sim:hover { border-color: var(--clr-accent); }
.sim.on { border-color: var(--clr-accent); background: var(--clr-accent-light); }
.sim-l { font-size: 11px; font-weight: 700; letter-spacing: .04em; text-transform: uppercase; color: var(--clr-text-muted); }
.sim.on .sim-l { color: var(--clr-accent); }
.sim-v { font-size: 1.35rem; font-weight: 780; letter-spacing: -.02em; }
.sim-s { font-size: 11px; color: var(--clr-text-muted); }

.ceil { display: flex; flex-direction: column; gap: .5rem; }
.crow { display: grid; grid-template-columns: 52px 1fr 92px; align-items: center; gap: 12px; }
.cmth { font-size: 12px; color: var(--clr-text-muted); font-weight: 600; }
.ctrack { position: relative; height: 12px; border-radius: 6px; background: var(--clr-track); overflow: hidden; }
.cfill { position: absolute; left: 0; top: 0; height: 100%; border-radius: 6px; }
.cfill.ok { background: var(--clr-positive); }
.cfill.over { background: var(--clr-negative); }
.cmark { position: absolute; top: -2px; bottom: -2px; width: 2px; background: var(--clr-text-primary); opacity: .55; }
.camt { font-size: 12.5px; font-weight: 700; text-align: right; }
.camt.ok { color: var(--clr-text-primary); }
.camt.over { color: var(--clr-negative); }

/* ranking */
.bars { display: flex; flex-direction: column; gap: .5rem; }
.brow { display: flex; justify-content: space-between; font-size: 12.5px; margin-bottom: 3px; }
.nm { color: var(--clr-text-secondary); font-weight: 600; } .amt { color: var(--clr-text-primary); font-weight: 700; }
.track { height: 8px; border-radius: 5px; background: var(--clr-track); overflow: hidden; }
.fill { height: 100%; border-radius: 5px; background: var(--clr-accent-hover); }
.bar.lead .fill { background: var(--clr-accent); }
.fill.other { background: var(--clr-text-muted); opacity: .5; }

/* fixo vs var */
.segbar { height: 16px; border-radius: 8px; overflow: hidden; display: flex; background: var(--clr-track); margin-bottom: .7rem; }
.segbar .s1 { background: var(--clr-accent); } .segbar .s2 { background: var(--clr-amber); } .segbar .s3 { background: var(--clr-negative); } .segbar .s4 { background: var(--clr-violet, #8b5cf6); }
.dotc.ded { background: var(--clr-negative); } .dotc.avul { background: var(--clr-violet, #8b5cf6); }
.leg { display: flex; gap: 1rem; font-size: 12px; flex-wrap: wrap; }
.leg span { display: flex; align-items: center; gap: .4rem; color: var(--clr-text-secondary); }
.dotc { width: 10px; height: 10px; border-radius: 3px; display: inline-block; }
.dotc.fix { background: var(--clr-accent); } .dotc.var { background: var(--clr-amber); }

@media (max-width: 760px) { .row2 { grid-template-columns: 1fr; } }

/* ── Matriz-seletor categoria × ano ── */
.mxbar { display: flex; align-items: flex-start; gap: 14px; flex-wrap: wrap; margin-bottom: 8px; }
.mxtools { margin-left: auto; display: inline-flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.tbtn { font-family: inherit; font-size: 12px; font-weight: 700; padding: 6px 12px; border-radius: var(--radius-md);
  border: 1px solid var(--clr-stroke); background: var(--clr-surface); color: var(--clr-text-secondary); cursor: pointer; }
.tbtn:hover { border-color: var(--clr-accent); color: var(--clr-accent); }
.mxtools .yrs { display: inline-flex; gap: 6px; align-items: center; font-size: 12px; font-weight: 600; color: var(--clr-text-secondary); }
.mxtools .yrs select { font-family: inherit; font-size: 13px; font-weight: 600; padding: 6px 10px;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer; }
.mxtools .dash { color: var(--clr-text-muted); }
.mxwrap { overflow-x: auto; border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); }
table.mx { border-collapse: collapse; width: 100%; font-size: 13px; min-width: 480px; font-variant-numeric: tabular-nums; }
table.mx th, table.mx td { padding: 9px 12px; text-align: right; border-bottom: 1px solid var(--clr-stroke-soft); white-space: nowrap; }
table.mx thead th { font-size: 11px; text-transform: uppercase; letter-spacing: .03em; color: var(--clr-text-muted); font-weight: 700; background: var(--clr-surface-alt); }
table.mx td:first-child, table.mx th:first-child { text-align: left; position: sticky; left: 0; background: var(--clr-surface); }
table.mx thead th:first-child { background: var(--clr-surface-alt); }
table.mx tbody tr { cursor: pointer; }
table.mx tbody tr:not(.tot):hover td { background: var(--clr-surface-alt); }
table.mx tbody tr:not(.tot):hover td:first-child { background: var(--clr-surface-alt); }
table.mx tr:last-child td { border-bottom: none; }
table.mx tr.tot td { font-weight: 800; background: var(--clr-surface-alt); }
table.mx tr.sel td:first-child { box-shadow: inset 3px 0 0 var(--clr-accent); font-weight: 700; color: var(--clr-text-primary); }
.mx .cell { border-radius: 6px; padding: 3px 8px; display: inline-block; min-width: 60px; }
.mx .cell.muted { color: var(--clr-text-muted); background: none; }
.mx .catname { display: inline-flex; align-items: center; gap: 8px; }
.mx .ck { width: 16px; height: 16px; border-radius: 4px; border: 1.5px solid var(--clr-stroke); display: inline-grid; place-items: center; font-size: 10px; color: #fff; flex: none; }
.mx .ck.on { background: var(--clr-accent); border-color: var(--clr-accent); }
.heat-legend { display: flex; align-items: center; gap: 8px; font-size: 11.5px; color: var(--clr-text-muted); margin-top: 10px; }
.heat-bar { height: 10px; width: 110px; border-radius: 5px;
  background: linear-gradient(90deg, color-mix(in srgb, var(--clr-accent) 10%, transparent), var(--clr-accent)); }

.selkpis { display: flex; gap: 12px; flex-wrap: wrap; margin-bottom: 12px; }
.skpi { border: 1px solid var(--clr-stroke); border-radius: 10px; padding: 10px 14px; min-width: 130px; }
.skpi .l { display: block; font-size: 11.5px; color: var(--clr-text-secondary); font-weight: 600; }
.skpi .v { display: block; font-size: 18px; font-weight: 800; letter-spacing: -.02em; margin-top: 2px; }
.skpi .s { display: block; font-size: 11px; color: var(--clr-text-muted); }
</style>
