<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { BarChart } from "echarts/charts";
import { GridComponent, TooltipComponent, LegendComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import { getAllTransactions, listBankEntries } from "@/services/tauri.service";
import type { BankEntry, Transaction } from "@/types/api.types";

use([CanvasRenderer, BarChart, GridComponent, TooltipComponent, LegendComponent]);

const bank = ref<BankEntry[]>([]);
const txs = ref<Transaction[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const isDark = ref(false);
const mql = window.matchMedia("(prefers-color-scheme: dark)");
const sync = () => (isDark.value = mql.matches);

const n = (s: string) => parseFloat(s) || 0;
const MONTHS = ["Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez"];
const monthLabel = (ym: string) => { const [y, m] = ym.split("-"); return `${MONTHS[parseInt(m, 10) - 1] ?? m}/${y.slice(2)}`; };
const brl = (v: number) => "R$ " + Math.round(v).toLocaleString("pt-BR");
const brlF = (v: number) => v.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });

onMounted(async () => {
  sync(); mql.addEventListener("change", sync);
  try {
    bank.value = await listBankEntries();
    txs.value = await getAllTransactions();
  } catch (e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { loading.value = false; }
});

const cardMonth = (t: Transaction) => t.date.slice(0, 7); // group card by transaction date

// KPIs
const entradas = computed(() => bank.value.filter((b) => n(b.amount) > 0).reduce((a, b) => a + n(b.amount), 0));
const saidas = computed(() => bank.value.filter((b) => n(b.amount) < 0).reduce((a, b) => a + Math.abs(n(b.amount)), 0));
const cartao = computed(() => txs.value.reduce((a, t) => a + (t.is_reversal ? -n(t.amount) : n(t.amount)), 0));
const saldoExtrato = computed(() => entradas.value - saidas.value);

const hasData = computed(() => bank.value.length > 0 || txs.value.length > 0);

// Per-month series
const months = computed(() => {
  const s = new Set<string>();
  bank.value.forEach((b) => s.add(b.month));
  txs.value.forEach((t) => s.add(cardMonth(t)));
  return [...s].filter(Boolean).sort();
});
const perMonth = computed(() => {
  const cred: Record<string, number> = {}, deb: Record<string, number> = {}, card: Record<string, number> = {};
  for (const b of bank.value) {
    const v = n(b.amount);
    if (v > 0) cred[b.month] = (cred[b.month] ?? 0) + v;
    else deb[b.month] = (deb[b.month] ?? 0) + Math.abs(v);
  }
  for (const t of txs.value) {
    const m = cardMonth(t);
    card[m] = (card[m] ?? 0) + (t.is_reversal ? -n(t.amount) : n(t.amount));
  }
  return { cred, deb, card };
});

const chartOption = computed(() => {
  const axis = isDark.value ? "#8aa39b" : "#5b6f68";
  const split = isDark.value ? "rgba(255,255,255,.08)" : "rgba(16,32,27,.08)";
  const tipBg = isDark.value ? "#14201d" : "#fff";
  const tipInk = isDark.value ? "#e8f0ed" : "#10201b";
  const accent = isDark.value ? "#34c9a6" : "#0e7c66";
  const red = isDark.value ? "#f0827b" : "#b3261e";
  const blue = isDark.value ? "#60a5fa" : "#3b82f6";
  const ms = months.value;
  const pm = perMonth.value;
  return {
    color: [accent, red, blue],
    tooltip: { trigger: "axis", backgroundColor: tipBg, borderColor: split, borderWidth: 1, textStyle: { color: tipInk, fontSize: 12 }, valueFormatter: (v: number) => brlF(v) },
    legend: { data: ["Entradas (extrato)", "Saídas (extrato)", "Cartão"], top: 0, textStyle: { color: axis, fontSize: 12 }, itemWidth: 14, itemHeight: 8 },
    grid: { left: 58, right: 16, top: 34, bottom: 28 },
    xAxis: { type: "category", data: ms.map(monthLabel), axisLabel: { color: axis, fontSize: 11, rotate: ms.length > 8 ? 40 : 0 }, axisLine: { lineStyle: { color: split } }, axisTick: { show: false } },
    yAxis: { type: "value", axisLabel: { color: axis, fontSize: 11, formatter: (v: number) => (v >= 1000 ? v / 1000 + "k" : "" + v) }, splitLine: { lineStyle: { color: split } } },
    series: [
      { name: "Entradas (extrato)", type: "bar", data: ms.map((m) => pm.cred[m] ?? 0), barMaxWidth: 18, itemStyle: { borderRadius: [3, 3, 0, 0] } },
      { name: "Saídas (extrato)", type: "bar", data: ms.map((m) => pm.deb[m] ?? 0), barMaxWidth: 18, itemStyle: { borderRadius: [3, 3, 0, 0] } },
      { name: "Cartão", type: "bar", data: ms.map((m) => pm.card[m] ?? 0), barMaxWidth: 18, itemStyle: { borderRadius: [3, 3, 0, 0] } },
    ],
  };
});

// Combined expense ranking: card by category + bank debits by category
const ranking = computed(() => {
  const map: Record<string, number> = {};
  for (const t of txs.value) map[t.category] = (map[t.category] ?? 0) + (t.is_reversal ? -n(t.amount) : n(t.amount));
  for (const b of bank.value) if (n(b.amount) < 0) map[b.category] = (map[b.category] ?? 0) + Math.abs(n(b.amount));
  return Object.entries(map).map(([name, value]) => ({ name, value })).filter((x) => x.value > 0).sort((a, b) => b.value - a.value).slice(0, 10);
});
const rankMax = computed(() => ranking.value[0]?.value || 1);
</script>

<template>
  <div class="page">
    <header class="top">
      <p class="eyebrow">Movimentações · extrato + cartão</p>
      <h1>Extratos &amp; Faturas</h1>
      <p class="sub">Panorama do que entra e sai nas suas contas (extrato) e do que passa no cartão (faturas).</p>
    </header>

    <div v-if="loading" class="state">Carregando…</div>
    <div v-else-if="error" class="state err">⚠ {{ error }}</div>
    <div v-else-if="!hasData" class="state">Importe faturas e/ou um extrato em <strong>Importações</strong> para ver o panorama.</div>

    <template v-else>
      <div class="kpis">
        <div class="kpi"><span class="l">Entradas (extrato)</span><span class="v pos">{{ brl(entradas) }}</span></div>
        <div class="kpi"><span class="l">Saídas (extrato)</span><span class="v neg">{{ brl(saidas) }}</span></div>
        <div class="kpi"><span class="l">Saldo do extrato</span><span class="v" :class="saldoExtrato >= 0 ? 'pos' : 'neg'">{{ saldoExtrato >= 0 ? "" : "− " }}{{ brl(Math.abs(saldoExtrato)) }}</span></div>
        <div class="kpi"><span class="l">Cartão (faturas)</span><span class="v">{{ brl(cartao) }}</span></div>
      </div>

      <div class="card">
        <h2>Mês a mês</h2>
        <p class="hint">Entradas e saídas do extrato + gasto no cartão, por mês.</p>
        <VChart :option="chartOption" autoresize style="height: 340px" />
      </div>

      <div class="card">
        <h2>Para onde foi o dinheiro</h2>
        <p class="hint">Despesas do cartão + débitos do extrato, por categoria.</p>
        <div class="bars">
          <div v-for="c in ranking" :key="c.name" class="bar">
            <div class="brow"><span class="nm">{{ c.name }}</span><span class="amt">{{ brlF(c.value) }}</span></div>
            <div class="track"><div class="fill" :style="{ width: (c.value / rankMax * 100) + '%' }"></div></div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.page { max-width: 1000px; margin: 0 auto; padding: 8px 4px 60px; }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--clr-accent); font-weight: 700; margin: 0 0 6px; }
h1 { font-size: 1.5rem; font-weight: 800; letter-spacing: -.02em; margin: 0; }
.sub { color: var(--clr-text-secondary); font-size: 14px; margin: 8px 0 16px; }
.state { padding: 40px 0; color: var(--clr-text-secondary); } .state.err { color: var(--clr-negative); }
.kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-bottom: 16px; }
.kpi { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: 12px; padding: 13px 15px; box-shadow: var(--shadow-sm); display: flex; flex-direction: column; gap: 3px; }
.kpi .l { font-size: 11.5px; color: var(--clr-text-secondary); font-weight: 600; }
.kpi .v { font-size: 21px; font-weight: 800; letter-spacing: -.02em; font-variant-numeric: tabular-nums; }
.kpi .v.pos { color: var(--clr-accent); } .kpi .v.neg { color: var(--clr-negative); }
.card { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: 14px; box-shadow: var(--shadow-sm); padding: 18px 20px; margin-bottom: 16px; }
.card h2 { font-size: 1rem; font-weight: 800; margin: 0 0 2px; }
.hint { font-size: 12.5px; color: var(--clr-text-muted, #7c8b83); margin: 0 0 12px; }
.bars { display: flex; flex-direction: column; gap: 10px; }
.bar .brow { display: flex; justify-content: space-between; font-size: 13px; margin-bottom: 4px; }
.bar .nm { color: var(--clr-text-secondary); } .bar .amt { font-weight: 700; font-variant-numeric: tabular-nums; }
.track { height: 9px; border-radius: 5px; background: var(--clr-surface-alt, #eef1f0); overflow: hidden; }
.fill { height: 100%; background: var(--clr-accent); border-radius: 5px; }
</style>
