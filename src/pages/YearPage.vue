<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from "vue";
import VChart from "vue-echarts";
import { use } from "echarts/core";
import { LineChart } from "echarts/charts";
import { GridComponent, TooltipComponent, LegendComponent } from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";
import { getYearSummary } from "@/services/tauri.service";
import type { YearSummary } from "@/types/api.types";

use([CanvasRenderer, LineChart, GridComponent, TooltipComponent, LegendComponent]);

const data = ref<YearSummary | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);
const selectedYear = ref<number | null>(null); // null = todos os anos
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
    data.value = await getYearSummary(selectedYear.value ?? undefined);
    // Keep the year list stable (backend returns all years regardless of filter).
    if (data.value.available_years.length) years.value = data.value.available_years;
    rangeFrom.value = null; // reset interval on (re)load
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
  const labels = ms.map((m) => monthLabel(m.month));
  return {
    color: [teal, coral],
    tooltip: {
      trigger: "axis",
      backgroundColor: tipBg,
      borderColor: split,
      borderWidth: 1,
      textStyle: { color: tipInk, fontSize: 12 },
      formatter: (ps: any[]) => {
        const i = ps[0].dataIndex;
        const m = ms[i];
        const bal = n(m.balance);
        const sign = bal >= 0 ? "+" : "−";
        return `<b>${monthLabel(m.month)}</b><br/>`
          + `Receita: ${brlF(m.income)}<br/>`
          + `Despesa: ${brlF(m.expense)}<br/>`
          + `<span style="color:${axis}">Saldo: ${sign}${brlF(Math.abs(bal))}</span>`;
      },
    },
    legend: { data: ["Receita", "Despesa"], top: 0, textStyle: { color: axis, fontSize: 12 }, itemWidth: 16, itemHeight: 3 },
    grid: { left: 58, right: 18, top: 34, bottom: 28 },
    xAxis: {
      type: "category",
      data: labels,
      axisLabel: { color: axis, fontSize: 11 },
      axisLine: { lineStyle: { color: split } },
      axisTick: { show: false },
    },
    yAxis: {
      type: "value",
      axisLabel: { color: axis, fontSize: 11, formatter: (v: number) => (v >= 1000 ? v / 1000 + "k" : "" + v) },
      splitLine: { lineStyle: { color: split } },
    },
    series: [
      { name: "Receita", type: "line", smooth: false, data: ms.map((m) => n(m.income)), lineStyle: { width: 2.5 }, symbol: "circle", symbolSize: 6 },
      { name: "Despesa", type: "line", smooth: false, data: ms.map((m) => n(m.expense)), lineStyle: { width: 2.5 }, symbol: "circle", symbolSize: 6,
        areaStyle: { color: coral, opacity: isDark.value ? 0.12 : 0.10 } },
    ],
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
      <label class="yearfilter" v-if="years.length">
        <span>Ano</span>
        <select v-model="selectedYear" @change="onYearChange">
          <option :value="null">Todos</option>
          <option v-for="y in years" :key="y" :value="y">{{ y }}</option>
        </select>
      </label>
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
          <span class="sub2">cartão {{ brl(data.card_total) }} + fixos {{ brl(data.fixed_total) }}</span>
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
          </div>
          <div class="leg">
            <span><i class="dotc fix"></i> Fixo · {{ brl(data.fixed_total) }}</span>
            <span><i class="dotc var"></i> Variável · {{ brl(data.card_total) }}</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.page { padding: 1.75rem 2rem 4rem; max-width: 1320px; margin: 0 auto; color: var(--clr-text-primary); font-variant-numeric: tabular-nums; }
.top { margin-bottom: 1.25rem; display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
.yearfilter { display: flex; align-items: center; gap: .5rem; font-size: 12px; font-weight: 600; color: var(--clr-text-secondary); }
.yearfilter select {
  font-family: inherit; font-size: 13px; font-weight: 600; padding: 6px 10px;
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-md);
  background: var(--clr-surface); color: var(--clr-text-primary); cursor: pointer; outline: none;
}
.yearfilter select:focus { border-color: var(--clr-accent); }
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
.segbar .s1 { background: var(--clr-accent); } .segbar .s2 { background: var(--clr-amber); }
.leg { display: flex; gap: 1rem; font-size: 12px; flex-wrap: wrap; }
.leg span { display: flex; align-items: center; gap: .4rem; color: var(--clr-text-secondary); }
.dotc { width: 10px; height: 10px; border-radius: 3px; display: inline-block; }
.dotc.fix { background: var(--clr-accent); } .dotc.var { background: var(--clr-amber); }

@media (max-width: 760px) { .row2 { grid-template-columns: 1fr; } }
</style>
