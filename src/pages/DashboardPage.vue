<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { useInvoiceStore } from "@/stores/invoice.store";
import { useSettingsStore } from "@/stores/settings.store";
import ImportButton from "@/components/import/ImportButton.vue";
import ImportWarnings from "@/components/import/ImportWarnings.vue";
import ReportOverlay from "@/components/report/ReportOverlay.vue";
import CategoryTreemap from "@/components/dashboard/CategoryTreemap.vue";
import type { Category, ManualEntry, ParseWarning, Payslip } from "@/types/api.types";
import { listPayslips } from "@/services/tauri.service";

const store = useInvoiceStore();
const settingsStore = useSettingsStore();
const lastWarnings = ref<ParseWarning[]>([]);

const d = computed(() => store.dashboard);
const num = (v?: string) => parseFloat(v ?? "0") || 0;

// "Todos os meses" sums the whole period. avgMode divides period aggregates by the
// number of months so figures read as a typical month; Total keeps period sums.
const isAllMonths = computed(() => !store.monthFilter);
const monthCount = computed(() => Math.max(1, d.value?.monthly_trend?.length ?? 0));
const avgMode = ref(false);
const showAvgToggle = computed(() => isAllMonths.value && monthCount.value > 1);
// Divides period-aggregate values (from the dashboard) when showing the monthly average.
const divisor = computed(() => (showAvgToggle.value && avgMode.value ? monthCount.value : 1));
// Multiplies already-monthly values (raw manual entries) up to the period total in Total mode.
const utilMult = computed(() => (showAvgToggle.value && !avgMode.value ? monthCount.value : 1));

const income = computed(() => num(d.value?.total_income) / divisor.value);
const expense = computed(() => num(d.value?.net_total) / divisor.value);
const balance = computed(() => num(d.value?.balance) / divisor.value);
const cardNet = computed(() => num(d.value?.total_card_net) / divisor.value);
const fixo = computed(() => num(d.value?.total_manual_expense) / divisor.value);
const futureParcelas = computed(() => num(d.value?.installments_future_total));
const monthParcelas = computed(() => num(d.value?.installments_month_total));
const balancePositive = computed(() => balance.value >= 0);
const savingsRate = computed(() => (income.value > 0 ? (balance.value / income.value) * 100 : null));

const cardPct = computed(() => (expense.value > 0 ? (cardNet.value / expense.value) * 100 : 0));
const fixoPct = computed(() => (expense.value > 0 ? (fixo.value / expense.value) * 100 : 0));
const payrollPct = computed(() => (expense.value > 0 ? (payrollDed.value / expense.value) * 100 : 0));
const avulsoPct = computed(() => (expense.value > 0 ? (variableExpense.value / expense.value) * 100 : 0));

const categories = computed<Category[]>(() =>
  (d.value?.categories ?? []).map((c) => ({ ...c, net_total: String(num(c.net_total) / divisor.value) }))
);
const catMax = computed(() => Math.max(1, ...categories.value.map((c) => num(c.net_total))));
const topTransactions = computed(() => d.value?.top_transactions ?? []);
const topMax = computed(() => Math.max(1, ...topTransactions.value.map((t) => num(t.amount))));

const expenseEntries = computed(() => store.manualEntries.filter((e) => e.kind === "expense"));
const fixedEntries = computed(() => expenseEntries.value.filter((e) => e.recurring));
const avulsoList = computed(() =>
  store.manualEntries.filter((e) => !e.recurring && (!store.monthFilter || e.month === store.monthFilter))
);
const variableExpense = computed(() => num(d.value?.total_variable_expense) / divisor.value);
const avulsoIncome = computed(
  () => avulsoList.value.filter((e) => e.kind === "income").reduce((s, e) => s + num(e.amount), 0) / divisor.value
);
const fixoCategories = computed(() => new Set(expenseEntries.value.map((e) => e.category)));

// ── Payslip data for the selected scope (salary líquido, deductions) ──
const payslips = ref<Payslip[]>([]);
const scopePayslips = computed(() =>
  store.monthFilter ? payslips.value.filter((p) => p.month === store.monthFilter) : payslips.value
);
const hasPayslip = computed(() => scopePayslips.value.length > 0);
const salaryNet = computed(() => scopePayslips.value.reduce((a, p) => a + num(p.net), 0) / divisor.value);
const salaryDed = computed(() => scopePayslips.value.reduce((a, p) => a + num(p.deductions), 0) / divisor.value);
const payrollDed = computed(() => num(d.value?.total_payroll_deductions) / divisor.value);
// Payroll deductions as detail rows (aggregated by description across scope payslips).
const deductionRows = computed(() => {
  const m = new Map<string, number>();
  for (const p of scopePayslips.value) {
    for (const it of p.items) {
      if (it.kind === "desconto" && !it.offsetting) {
        m.set(it.description, (m.get(it.description) ?? 0) + num(it.amount));
      }
    }
  }
  return [...m.entries()]
    .map(([description, amount]) => ({ description, amount: amount / divisor.value }))
    .sort((a, b) => b.amount - a.amount);
});
// Extra (non-salary) manual income in scope — e.g. bolsa. = total income − payslip gross.
const payslipGross = computed(() => scopePayslips.value.reduce((a, p) => a + num(p.real_gross), 0) / divisor.value);
const bolsaIncome = computed(() => Math.max(0, income.value - payslipGross.value));
// Bonus / gratificações (CD, férias…) — líquido attributable to each, from the payslip.
const bonusPayslip = computed(() => scopePayslips.value.reduce((a, p) => a + num(p.bonus_liq), 0) / divisor.value);
// Total bonus/extra income = payslip bonuses + extra manual income (bolsa).
const bonusLiq = computed(() => bonusPayslip.value + bolsaIncome.value);
const bonusRows = computed(() => {
  const m = new Map<string, number>();
  for (const p of scopePayslips.value) {
    for (const it of p.items) {
      if (it.kind === "rendimento" && it.class === "bonus" && !it.offsetting) {
        m.set(it.description, (m.get(it.description) ?? 0) + num(it.net_share));
      }
    }
  }
  return [...m.entries()]
    .map(([description, amount]) => ({ description, amount: amount / divisor.value }))
    .sort((a, b) => b.amount - a.amount);
});

// Card ceiling for the month (renda recorrente / só salário − contas fixas).
const salaryLiqMonth = computed(() => Math.max(0, salaryNet.value - bonusPayslip.value));
const tetoRecorrente = computed(() => Math.max(0, salaryNet.value + bolsaIncome.value - fixo.value));
const tetoSalario = computed(() => Math.max(0, salaryLiqMonth.value - fixo.value));
const cardOverCeiling = computed(() => cardNet.value > tetoRecorrente.value);

// ── Report overlay ──
const reportOpen = ref(false);
const reportTitle = computed(() =>
  isAllMonths.value ? `Relatório · ${periodLabel()}` : `Relatório de ${formatMonthFilter(store.monthFilter ?? "")}`
);
const genDate = computed(() => new Date().toLocaleDateString("pt-BR"));
const avulsoExpenses = computed(() => avulsoList.value.filter((e) => e.kind === "expense"));
const topCats = computed(() => categories.value.slice(0, 8));
const treemapItems = computed(() => categories.value.map((c) => ({ name: c.name, value: num(c.net_total) })));

// ── Drill-down: click a category to list its expenses (card + fixos + avulsos + folha) ──
const expandedCat = ref<string | null>(null);
function toggleCatDrill(name: string): void { expandedCat.value = expandedCat.value === name ? null : name; }
const scopeInvoiceIds = computed(() => {
  const invs = store.monthFilter ? store.invoices.filter((i) => i.month === store.monthFilter) : store.invoices;
  return new Set(invs.map((i) => i.id));
});
// Mirror of backend deduction_category so payroll rows land under the right category.
function dedCat(desc: string): string {
  const u = desc.toUpperCase();
  if (u.includes("IMPOSTO") || u.includes("IRRF") || u.includes("RENDA")) return "Impostos";
  if (u.includes("GEAP") || u.includes("SAUDE") || u.includes("SAÚDE") || u.includes("PSAUDE") || u.includes("PSAÚDE")) return "Saúde";
  if (u.includes("FUNPRESP") || u.includes("SEGURIDADE") || u.includes("PSS") || u.includes("PREVID")) return "Previdência";
  return "Descontos da folha";
}
function fmtDay(d: string): string { const p = d.split("-"); return p.length === 3 ? `${p[2]}/${p[1]}` : d; }
function monthShort(m: string): string { const [y, mo] = m.split("-"); return `${MONTHS[parseInt(mo, 10) - 1] ?? mo}/${y.slice(2)}`; }
type DrillItem = { date: string; desc: string; amount: number; source: "card" | "fix" | "avul" | "folha"; reversal?: boolean };
const drillItems = computed<DrillItem[]>(() => {
  const cat = expandedCat.value;
  if (!cat) return [];
  const out: DrillItem[] = [];
  for (const t of store.allTransactions) {
    if (t.category !== cat || !scopeInvoiceIds.value.has(t.invoice_id)) continue;
    out.push({ date: fmtDay(t.date), desc: t.description, amount: num(t.amount) * (t.is_reversal ? -1 : 1), source: "card", reversal: t.is_reversal });
  }
  const inScopeMonth = (m: string) => !store.monthFilter || m === store.monthFilter;
  for (const e of store.manualEntries) {
    if (e.kind !== "expense" || e.category !== cat) continue;
    if (e.recurring) out.push({ date: "mensal", desc: e.description, amount: num(e.amount), source: "fix" });
    else if (inScopeMonth(e.month)) out.push({ date: monthShort(e.month), desc: e.description, amount: num(e.amount), source: "avul" });
  }
  for (const p of scopePayslips.value) {
    for (const it of p.items) {
      if (it.kind !== "desconto" || it.offsetting || dedCat(it.description) !== cat) continue;
      out.push({ date: monthShort(p.month), desc: it.description, amount: num(it.amount), source: "folha" });
    }
  }
  return out.sort((a, b) => b.amount - a.amount);
});
const drillTotal = computed(() => drillItems.value.reduce((s, i) => s + i.amount, 0));
const SRC_LABEL: Record<DrillItem["source"], string> = { card: "cartão", fix: "fixo", avul: "avulso", folha: "folha" };

const UTIL_RE = /energ|[aá]gua|luz|saneam/i;
const AGUA_RE = /[aá]gua|saneam/i;
const ENERGIA_RE = /energ|luz/i;
function isUtil(desc: string) { return UTIL_RE.test(desc); }
const aguaAmt = computed(() =>
  expenseEntries.value.filter((e) => AGUA_RE.test(e.description)).reduce((a, e) => a + num(e.amount), 0)
);
const energiaAmt = computed(() =>
  expenseEntries.value.filter((e) => ENERGIA_RE.test(e.description)).reduce((a, e) => a + num(e.amount), 0)
);
// aguaAmt/energiaAmt are already monthly (raw entries); scale up to the period in Total mode.
const utilities = computed(() => (aguaAmt.value + energiaAmt.value) * utilMult.value);
const utilitiesHigh = computed(() => aguaAmt.value >= 300 || energiaAmt.value >= 500);

const outros = computed(() => categories.value.find((c) => c.name === "Outros"));
const outrosPct = computed(() => outros.value?.percentage ?? 0);
const moradia = computed(() =>
  categories.value.find((c) => /moradia/i.test(c.name))
);

// weekday: Mon..Sun
const WD_LABELS = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
const weekday = computed(() => (d.value?.weekday_spending ?? []).map((v) => num(v) / divisor.value));
const weekdayMax = computed(() => Math.max(1, ...weekday.value));
const weekTotal = computed(() => weekday.value.reduce((a, b) => a + b, 0));
const weekendPct = computed(() => {
  const wknd = (weekday.value[4] ?? 0) + (weekday.value[5] ?? 0);
  return weekTotal.value > 0 ? (wknd / weekTotal.value) * 100 : 0;
});

const installments = computed(() => (d.value?.installments ?? []).slice(0, 8));
const instMax = computed(() => Math.max(1, ...installments.value.map((i) => num(i.amount))));

const subscriptions = computed(() => d.value?.subscriptions ?? []);
const subsTotal = computed(() => num(d.value?.subscriptions_total));
const subsAnomaly = computed(() => subscriptions.value.find((s) => s.count >= 3));

const hasComposition = computed(() => fixo.value > 0 || income.value > 0);

// month/year filter
const availableMonths = computed(() =>
  [...new Set(store.invoices.map((i) => i.month).filter(Boolean))].sort().reverse()
);
function onMonthChange(e: Event): void {
  const v = (e.target as HTMLSelectElement).value;
  store.setMonthFilter(v || null);
}

// ── suggestions (rule-based, generated from live data) ──
interface Suggestion { title: string; tag: string; pri: "red" | "amber" | "accent"; body: string; impact: string; min?: number; max?: number; }
const suggestions = computed<Suggestion[]>(() => {
  const s: Suggestion[] = [];
  if (aguaAmt.value >= 300) {
    s.push({
      title: `Água ${fmt0(aguaAmt.value)} → caçar vazamento`,
      tag: "Maior ROI", pri: "red",
      body: "Conta de água muito acima da média de uma casa. Teste do hidrômetro: feche tudo por 1h e veja se o relógio gira. Cheque descarga/caixa d'água e peça revisão de leitura.",
      impact: `Ganho potencial: até ${fmt0(aguaAmt.value * 0.8)}/mês se for vazamento.`,
      min: aguaAmt.value * 0.6, max: aguaAmt.value * 0.9,
    });
  }
  if (energiaAmt.value >= 500) {
    s.push({
      title: `Energia ${fmt0(energiaAmt.value)} → auditar consumo`,
      tag: "Maior ROI", pri: "red",
      body: "Consumo alto ou tarifa/bandeira ruim. Verifique chuveiro elétrico, ar-condicionado e geladeira antiga. Avalie tarifa branca e, se cabível, energia solar.",
      impact: `Ganho potencial: ${fmt0(energiaAmt.value * 0.3)}–${fmt0(energiaAmt.value * 0.5)}/mês com ajuste de hábitos + equipamento.`,
      min: energiaAmt.value * 0.3, max: energiaAmt.value * 0.5,
    });
  }
  if (futureParcelas.value > 0) {
    s.push({
      title: "Congelar novas compras parceladas",
      tag: "Caixa futuro", pri: "amber",
      body: "Parcelas já contratadas travam a folga dos próximos meses. Cada nova parcela come o orçamento seguinte — que já está apertado com os fixos.",
      impact: `Ganho: até ${fmt0(futureParcelas.value)} de fôlego preservado.`,
    });
  }
  if (outrosPct.value >= 30) {
    s.push({
      title: `Enxergar os ${outrosPct.value.toFixed(0)}% em "Outros"`,
      tag: "Visibilidade", pri: "amber",
      body: 'Boa parte dos lançamentos do cartão está sem categoria (supermercado, açougue, pet, postos). Adicione regras em Configurações: SUPERMERCADO, MERCADO, ACOUGUE, HORTIFRUTI, PET.',
      impact: outros.value ? `Ganho: controle sobre ${fmt0(num(outros.value.net_total))} hoje invisíveis.` : "Ganho: visibilidade dos gastos.",
    });
  }
  if (subsTotal.value > 0) {
    const top = subscriptions.value.slice(0, 3).map((x) => `${x.name} ${fmt0(num(x.total))}`).join(", ");
    const anom = subsAnomaly.value
      ? ` ${subsAnomaly.value.name} aparece ${subsAnomaly.value.count}× no mês — parece cobrança avulsa, não assinatura.`
      : "";
    s.push({
      title: "Auditar assinaturas",
      tag: "Recorrente", pri: "accent",
      body: `${fmt0(subsTotal.value * 12)}/ano no cartão em recorrentes (${top}).${anom}`,
      impact: `Ganho: cancelar/ajustar ≈ ${fmt0(subsTotal.value * 0.4)}/mês → ${fmt0(subsTotal.value * 0.4 * 12)}/ano.`,
      min: subsTotal.value * 0.3, max: subsTotal.value * 0.6,
    });
  }
  if (weekendPct.value >= 40) {
    const wknd = (weekday.value[4] ?? 0) + (weekday.value[5] ?? 0);
    s.push({
      title: "Teto de fim de semana",
      tag: "Comportamento", pri: "accent",
      body: `Sexta e sábado concentram ${weekendPct.value.toFixed(0)}% do gasto de cartão da semana. Definir um limite e usar débito/PIX no lugar do crédito freia o impulso.`,
      impact: `Ganho: corte de 15% ≈ ${fmt0(wknd * 0.15)}/mês.`,
      min: wknd * 0.1, max: wknd * 0.2,
    });
  }
  if (income.value > 0 && balance.value < 0) {
    s.push({
      title: "Saldo negativo no mês",
      tag: "Alerta", pri: "red",
      body: "As despesas superaram as receitas. Priorize cortar os fixos anômalos e segurar o cartão até reequilibrar.",
      impact: `Déficit atual: ${fmt0(Math.abs(balance.value))}.`,
    });
  }
  return s;
});
const potentialMin = computed(() => suggestions.value.reduce((a, s) => a + (s.min ?? 0), 0));
const potentialMax = computed(() => suggestions.value.reduce((a, s) => a + (s.max ?? 0), 0));
const hasPotential = computed(() => potentialMax.value > 0);

onMounted(async () => {
  await store.refreshInvoices();
  await settingsStore.loadConfig();
  await store.loadManualEntries();
  try { payslips.value = await listPayslips(); } catch { /* ignore */ }
  try { await store.loadAllTransactions(); } catch { /* ignore */ }
  if (store.hasData) await store.loadDashboard();
});

// Quick add: one-off debit/credit for the selected month (freelance, an unexpected bill…).
const qaOpen = ref(false);
const qaEditId = ref<string | null>(null);
const qaKind = ref<"expense" | "income">("expense");
const qaDesc = ref("");
const qaAmount = ref("");
const qaCat = ref("");
const qaError = ref<string | null>(null);
const qaSuggestions = computed(() =>
  qaKind.value === "income"
    ? ["Freelance", "Rendimentos", "Bolsa de Pesquisa", "Reembolso", "Outros"]
    : settingsStore.categoryGroups.map((g) => g.name)
);
function qaMonth(): string {
  if (store.monthFilter) return store.monthFilter;
  const dt = new Date();
  return `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, "0")}`;
}
async function addQuick(): Promise<void> {
  qaError.value = null;
  const amt = parseFloat(qaAmount.value.replace(",", "."));
  if (!qaDesc.value.trim()) { qaError.value = "Informe uma descrição."; return; }
  if (!qaCat.value.trim()) { qaError.value = "Informe uma categoria."; return; }
  if (!(amt > 0)) { qaError.value = "Informe um valor maior que zero."; return; }
  try {
    const input = {
      kind: qaKind.value,
      description: qaDesc.value.trim(),
      amount: String(amt),
      category: qaCat.value.trim(),
      month: qaMonth(),
      recurring: false,
      isSalary: false,
    };
    if (qaEditId.value) {
      await store.updateManualEntry(qaEditId.value, input);
    } else {
      await store.addManualEntry(input);
    }
    resetQuick();
    await store.loadManualEntries();
    await store.loadDashboard();
  } catch (e) {
    qaError.value = String(e instanceof Error ? e.message : e);
  }
}
function resetQuick(): void {
  qaEditId.value = null;
  qaDesc.value = ""; qaAmount.value = ""; qaCat.value = "";
  qaKind.value = "expense";
  qaOpen.value = false;
  qaError.value = null;
}
function toggleQuick(): void {
  if (qaOpen.value) { resetQuick(); } else { qaOpen.value = true; }
}
function editAvulso(e: ManualEntry): void {
  qaEditId.value = e.id;
  qaKind.value = e.kind;
  qaDesc.value = e.description;
  qaAmount.value = e.amount;
  qaCat.value = e.category;
  qaError.value = null;
  qaOpen.value = true;
}
async function removeAvulso(id: string): Promise<void> {
  try {
    await store.removeManualEntry(id);
    if (qaEditId.value === id) resetQuick();
    await store.loadManualEntries();
    await store.loadDashboard();
  } catch (e) {
    qaError.value = String(e instanceof Error ? e.message : e);
  }
}

// password prompt for encrypted BTG files
const pwPrompt = ref(false);
const pwPaths = ref<string[]>([]);
const pwValue = ref("");
const pwError = ref<string | null>(null);
const pwRemember = ref(true);

async function handleImport(paths: string[]): Promise<void> {
  try {
    // No password here: the backend silently reuses a saved keychain password if present.
    const results = await store.importInvoices(paths);
    lastWarnings.value = results.flatMap((r) => r.warnings);
    await store.loadDashboard();
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    // ENCRYPTED_FILE: nothing saved yet. WRONG_PASSWORD: saved one is stale — re-prompt either way.
    if (msg === "ENCRYPTED_FILE" || msg === "WRONG_PASSWORD") {
      pwPaths.value = paths;
      pwValue.value = "";
      pwError.value = msg === "WRONG_PASSWORD" ? "Senha salva inválida. Informe a senha novamente." : null;
      pwRemember.value = true;
      pwPrompt.value = true;
    }
    // other errors already surfaced by the store
  }
}

async function submitPassword(): Promise<void> {
  pwError.value = null;
  try {
    const results = await store.importInvoices(pwPaths.value, pwValue.value, pwRemember.value);
    lastWarnings.value = results.flatMap((r) => r.warnings);
    await store.loadDashboard();
    pwPrompt.value = false;
    pwValue.value = "";
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (msg === "WRONG_PASSWORD") pwError.value = "Senha incorreta. Tente novamente.";
    else if (msg === "ENCRYPTED_FILE") pwError.value = "Informe a senha do arquivo.";
    else pwError.value = msg;
  }
}

function cancelPassword(): void {
  pwPrompt.value = false;
  pwValue.value = "";
  pwError.value = null;
}

function fmt(val: number | string): string {
  const n = typeof val === "string" ? parseFloat(val) || 0 : val;
  return n.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
}
function fmt0(val: number): string {
  return val.toLocaleString("pt-BR", { style: "currency", currency: "BRL", maximumFractionDigits: 0 });
}
function kBRL(v: number): string {
  return (v / 1000).toLocaleString("pt-BR", { minimumFractionDigits: 1, maximumFractionDigits: 1 }) + "k";
}
function pctOf(v: number, max: number): number { return Math.max((v / max) * 100, 2); }

const MONTHS = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
const MONTHS_FULL = ["Janeiro","Fevereiro","Março","Abril","Maio","Junho","Julho","Agosto","Setembro","Outubro","Novembro","Dezembro"];
function periodLabel(): string {
  const p = d.value?.period;
  if (!p || !p.from) return "—";
  const [yf, mf] = p.from.split("-");
  const [yt, mt] = p.to.split("-");
  const a = `${MONTHS[parseInt(mf) - 1]}/${yf}`;
  const b = `${MONTHS[parseInt(mt) - 1]}/${yt}`;
  return p.from === p.to ? a : `${a} – ${b}`;
}
function formatMonthFilter(month: string): string {
  const [year, m] = month.split("-");
  return `${MONTHS_FULL[parseInt(m) - 1] ?? m}/${year}`;
}
function titleSuffix(): string {
  const p = d.value?.period;
  if (!p || !p.from) return "";
  if (!isAllMonths.value) {
    const mf = parseInt(p.from.split("-")[1]);
    return `em ${MONTHS_FULL[mf - 1]?.toLowerCase() ?? ""}`;
  }
  return avgMode.value ? "· média por mês" : `· período ${periodLabel()}`;
}
const scopeWord = computed(() => {
  if (!isAllMonths.value) return "do mês";
  return avgMode.value ? "média/mês" : "do período";
});
function refLabel(): string {
  const p = d.value?.period;
  if (!p || !p.from) return "—";
  if (isAllMonths.value && p.from !== p.to) {
    const [yf, mf] = p.from.split("-");
    const [yt, mt] = p.to.split("-");
    return `${MONTHS[parseInt(mf) - 1].toLowerCase()}/${yf} – ${MONTHS[parseInt(mt) - 1].toLowerCase()}/${yt}`;
  }
  const [y, mf] = p.from.split("-");
  return `${MONTHS[parseInt(mf) - 1].toLowerCase()}/${y}`;
}
</script>

<template>
  <div class="dash">
    <!-- Header -->
    <header class="top">
      <div class="top-row">
        <div>
          <p class="eyebrow">Análise de despesas domésticas · Casa + Cartão BTG</p>
          <h1>Custo total da casa{{ titleSuffix() ? " " + titleSuffix() : "" }}</h1>
        </div>
        <div class="top-actions">
          <select
            v-if="availableMonths.length"
            class="month-select"
            :value="store.monthFilter ?? ''"
            @change="onMonthChange"
            title="Filtrar por mês"
          >
            <option value="">Todos os meses</option>
            <option v-for="m in availableMonths" :key="m" :value="m">{{ formatMonthFilter(m) }}</option>
          </select>
          <div v-if="showAvgToggle" class="avg-toggle" role="group" aria-label="Total ou média">
            <button type="button" :class="{ active: !avgMode }" @click="avgMode = false">Total</button>
            <button type="button" :class="{ active: avgMode }" @click="avgMode = true">Média/mês</button>
          </div>
          <span v-else-if="d" class="period">{{ periodLabel() }}</span>
          <button class="qa-btn" @click="toggleQuick">+ Lançamento avulso</button>
          <button v-if="d" class="qa-btn" @click="reportOpen = true">📄 Relatório</button>
          <ImportButton @import-requested="handleImport" />
        </div>
      </div>
      <p v-if="d" class="sub">
        Cartão BTG (ref. {{ refLabel() }}) + contas fixas mensais. Total de <b>{{ fmt(expense) }}</b>:
        <b>{{ fmt(cardNet) }}</b> no cartão, <b>{{ fmt0(fixo) }}</b> em fixos<template v-if="payrollDed > 0"> e <b>{{ fmt0(payrollDed) }}</b> em descontos da folha</template>.
        Leitura de analista: peso, o que já está comprometido e onde cortar.
        <template v-if="income > 0"> Receitas <b>{{ fmt(income) }}</b> · saldo <b :class="balancePositive ? 'ok-text' : 'red-text'">{{ fmt(balance) }}</b>.</template>
      </p>
    </header>

    <!-- Quick add: one-off debit/credit for the month -->
    <div v-if="qaOpen" class="qa-card">
      <div class="qa-kind">
        <button type="button" :class="{ active: qaKind === 'expense' }" @click="qaKind = 'expense'">↓ Débito</button>
        <button type="button" :class="{ active: qaKind === 'income' }" @click="qaKind = 'income'">↑ Crédito</button>
      </div>
      <input v-model="qaDesc" class="qa-in" type="text" :placeholder="qaKind === 'income' ? 'Ex: Freelance' : 'Ex: Conta avulsa'" @keyup.enter="addQuick" />
      <input v-model="qaAmount" class="qa-in qa-amt" type="text" inputmode="decimal" placeholder="0,00" @keyup.enter="addQuick" />
      <input v-model="qaCat" class="qa-in" type="text" list="qa-cats" placeholder="Categoria" @keyup.enter="addQuick" />
      <datalist id="qa-cats"><option v-for="c in qaSuggestions" :key="c" :value="c" /></datalist>
      <span class="qa-mes">{{ formatMonthFilter(qaMonth()) }}</span>
      <button class="qa-add" :disabled="store.loading" @click="addQuick">{{ qaEditId ? "Salvar" : "Adicionar" }}</button>
      <button v-if="qaEditId" class="qa-cancel" :disabled="store.loading" @click="resetQuick">Cancelar</button>
      <span v-if="qaError" class="qa-err">⚠ {{ qaError }}</span>
    </div>

    <ImportWarnings :warnings="lastWarnings" />

    <!-- Password prompt for encrypted BTG files -->
    <div v-if="pwPrompt" class="pw-overlay" @click.self="cancelPassword">
      <div class="pw-modal">
        <h3>Fatura protegida por senha</h3>
        <p class="pw-sub">Este arquivo BTG está criptografado. Informe a senha para importar.</p>
        <input
          v-model="pwValue"
          type="password"
          class="pw-input"
          placeholder="Senha do arquivo"
          autofocus
          @keyup.enter="submitPassword"
        />
        <label class="pw-remember">
          <input type="checkbox" v-model="pwRemember" />
          <span>Lembrar senha neste dispositivo</span>
        </label>
        <div v-if="pwError" class="pw-err">⚠ {{ pwError }}</div>
        <div class="pw-actions">
          <button class="pw-btn ghost" @click="cancelPassword">Cancelar</button>
          <button class="pw-btn primary" :disabled="store.loading || !pwValue" @click="submitPassword">
            {{ store.loading ? "Abrindo…" : "Importar" }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="store.monthFilter" class="filter-badge">
      <span>Filtrado: <strong>{{ formatMonthFilter(store.monthFilter) }}</strong></span>
      <button class="clear-filter" @click="store.setMonthFilter(null)">✕ Limpar</button>
    </div>

    <div v-if="store.error" class="msg-error"><span>⚠</span>{{ store.error }}</div>

    <div v-if="store.loading" class="loading">
      <div class="shimmer" v-for="i in 8" :key="i" />
    </div>

    <template v-if="d && !store.loading">
      <!-- Indicadores -->
      <section>
        <h2>Indicadores</h2>
        <div class="kpis">
          <div :class="['kpi', 'flag', balancePositive ? 'flag-ok' : 'flag-red']">
            <p class="lbl">Saldo {{ scopeWord }}</p>
            <div class="val" :class="balancePositive ? 'ok-text' : 'red-text'">{{ fmt(balance) }}</div>
            <div class="foot" v-if="savingsRate !== null">{{ balancePositive ? "sobra" : "déficit" }} · {{ Math.abs(savingsRate).toFixed(0) }}% da receita</div>
            <div class="foot" v-else>cadastre receitas para ver o saldo</div>
          </div>
          <div class="kpi">
            <p class="lbl">Receitas</p>
            <div class="val ok-text">{{ fmt(income) }}</div>
            <div class="foot">salário bruto, bolsas, rendimentos</div>
          </div>
          <div v-if="hasPayslip" class="kpi">
            <p class="lbl">Salário líquido</p>
            <div class="val ok-text">{{ fmt(salaryNet) }}</div>
            <div class="foot">do contracheque</div>
          </div>
          <div v-if="hasPayslip" class="kpi flag flag-amber">
            <p class="lbl">Descontos do salário</p>
            <div class="val">{{ fmt(salaryDed) }}</div>
            <div class="foot">FUNPRESP, GEAP, PSS, IR</div>
          </div>
          <div v-if="bonusLiq > 0" class="kpi">
            <p class="lbl">Bônus + renda extra</p>
            <div class="val ok-text">{{ fmt(bonusLiq) }}</div>
            <div class="foot">CD, gratificações, bolsa, rendimentos</div>
          </div>
          <div class="kpi">
            <p class="lbl">Custo total {{ scopeWord }}</p>
            <div class="val">{{ fmt(expense) }}</div>
            <div class="foot">cartão {{ fmt0(cardNet) }} + fixos {{ fmt0(fixo) }}<template v-if="variableExpense > 0"> + avulsos {{ fmt0(variableExpense) }}</template><template v-if="payrollDed > 0"> + descontos {{ fmt0(payrollDed) }}</template></div>
          </div>
          <div class="kpi">
            <p class="lbl">Contas fixas {{ scopeWord }}</p>
            <div class="val">{{ fmt(fixo) }}</div>
            <div class="foot">{{ fixoPct.toFixed(0) }}% das despesas</div>
          </div>
          <div v-if="utilities > 0" :class="['kpi', 'flag', utilitiesHigh ? 'flag-red' : '']">
            <p class="lbl">Energia + Água</p>
            <div class="val" :class="utilitiesHigh ? 'red-text' : ''">{{ fmt(utilities) }}</div>
            <span v-if="utilitiesHigh" class="pill bad">▲ acima do normal</span>
            <div v-else class="foot">contas de utilidade</div>
          </div>
          <div v-if="futureParcelas > 0" class="kpi flag flag-amber">
            <p class="lbl">Comprometido em parcelas</p>
            <div class="val">{{ fmt(futureParcelas) }}</div>
            <span class="pill warn">◆ trava caixa futuro</span>
          </div>
          <div :class="['kpi', 'flag', outrosPct >= 35 ? 'flag-amber' : '']">
            <p class="lbl">Não categorizado</p>
            <div class="val">{{ outrosPct.toFixed(1) }}%</div>
            <span v-if="outrosPct >= 35" class="pill warn">◆ visibilidade</span>
            <div v-else class="foot">em "Outros"</div>
          </div>
          <div v-if="moradia" class="kpi">
            <p class="lbl">Moradia</p>
            <div class="val">{{ fmt(moradia.net_total) }}</div>
            <div class="foot">{{ moradia.percentage.toFixed(0) }}% do total</div>
          </div>
        </div>
      </section>

      <!-- Fixo x variavel -->
      <section v-if="hasComposition">
        <h2>Fixo × variável</h2>
        <div class="grid2">
          <div class="card">
            <h3>Composição do custo total</h3>
            <p class="cap">Fixo = contas recorrentes fora do cartão. Variável = tudo no cartão (inclui parcelas e assinaturas).</p>
            <div class="split">
              <div class="seg seg-card" :style="{ flexGrow: cardNet }" :title="`Cartão: ${fmt(cardNet)}`">
                <span v-if="cardPct > 12">Cartão {{ cardPct.toFixed(1) }}%</span>
              </div>
              <div class="seg seg-fixo" :style="{ flexGrow: fixo }" :title="`Fixos: ${fmt(fixo)}`">
                <span v-if="fixoPct > 12">Fixos {{ fixoPct.toFixed(1) }}%</span>
              </div>
              <div v-if="variableExpense > 0" class="seg seg-avulso" :style="{ flexGrow: variableExpense }" :title="`Avulsos: ${fmt(variableExpense)}`">
                <span v-if="avulsoPct > 12">Avulsos {{ avulsoPct.toFixed(1) }}%</span>
              </div>
              <div v-if="payrollDed > 0" class="seg seg-payroll" :style="{ flexGrow: payrollDed }" :title="`Descontos: ${fmt(payrollDed)}`">
                <span v-if="payrollPct > 12">Descontos {{ payrollPct.toFixed(1) }}%</span>
              </div>
            </div>
            <div class="legend">
              <span><i class="dot dot-card"></i> Cartão / variável — {{ fmt(cardNet) }}</span>
              <span><i class="dot dot-fixo"></i> Fixos — {{ fmt(fixo) }}</span>
              <span v-if="variableExpense > 0"><i class="dot dot-avulso"></i> Avulsos — {{ fmt(variableExpense) }}</span>
              <span v-if="payrollDed > 0"><i class="dot dot-payroll"></i> Descontos da folha — {{ fmt(payrollDed) }}</span>
            </div>
            <p class="note">Regra de bolso saudável: fixos ≤ 50% da renda. Fique de olho no <b>valor absoluto</b> de água e energia.</p>
          </div>

          <div class="card">
            <h3>Contas fixas — detalhe</h3>
            <p class="cap">Contas fixas mensais (sem descontos da folha). Água e energia sinalizadas como anomalia.</p>
            <div v-if="fixedEntries.length" class="fixlist">
              <div v-for="e in fixedEntries" :key="e.id" class="fixrow">
                <span class="fn" :class="{ hot: isUtil(e.description) && utilitiesHigh }">
                  {{ e.description }}
                  <span v-if="isUtil(e.description) && utilitiesHigh" class="badge-hot">alto</span>
                </span>
                <b :class="{ 'red-text': isUtil(e.description) && utilitiesHigh }">{{ fmt(e.amount) }}</b>
              </div>
              <div class="fixrow tot">
                <span class="fn">Total fixo</span>
                <b>{{ fmt(fixo) }}</b>
              </div>
            </div>
            <p v-else class="cap">Nenhuma conta fixa. Cadastre em <strong>Fixos &amp; Renda</strong>.</p>
          </div>

          <div class="card" v-if="avulsoList.length">
            <h3>Lançamentos avulsos — detalhe</h3>
            <p class="cap">Débitos e créditos avulsos (não-recorrentes) deste mês. Passe o mouse para editar ou remover.</p>
            <div class="fixlist">
              <div v-for="e in avulsoList" :key="e.id" class="fixrow avrow">
                <span class="fn">
                  <span class="badge-kind" :class="e.kind">{{ e.kind === "income" ? "crédito" : "débito" }}</span>
                  {{ e.description }}
                  <small class="av-cat">{{ e.category }}</small>
                </span>
                <span class="avright">
                  <b :class="{ 'green-text': e.kind === 'income' }">{{ e.kind === "income" ? "+" : "" }}{{ fmt(e.amount) }}</b>
                  <span class="avactions">
                    <button class="av-ic" title="Editar" @click="editAvulso(e)">✎</button>
                    <button class="av-ic del" title="Remover" @click="removeAvulso(e.id)">✕</button>
                  </span>
                </span>
              </div>
              <div class="fixrow tot">
                <span class="fn">Total despesas avulsas</span>
                <b>{{ fmt(variableExpense) }}</b>
              </div>
              <div v-if="avulsoIncome > 0" class="fixrow tot">
                <span class="fn">Total créditos avulsos</span>
                <b class="green-text">+{{ fmt(avulsoIncome) }}</b>
              </div>
            </div>
          </div>

          <div class="card" v-if="deductionRows.length">
            <h3>Descontos da folha — detalhe</h3>
            <p class="cap">FUNPRESP, GEAP (saúde), PSS (previdência) e IR retido — do contracheque.</p>
            <div class="fixlist">
              <div v-for="(dr, i) in deductionRows" :key="i" class="fixrow">
                <span class="fn">{{ dr.description }}</span>
                <b>{{ fmt(dr.amount) }}</b>
              </div>
              <div class="fixrow tot">
                <span class="fn">Total descontos</span>
                <b>{{ fmt(payrollDed) }}</b>
              </div>
            </div>
          </div>

          <div class="card" v-if="bonusRows.length || bolsaIncome > 0">
            <h3>Bônus &amp; renda extra — detalhe</h3>
            <p class="cap">Ganhos não-permanentes (líquido): cargo de direção, gratificações, férias, bolsa e rendimentos.</p>
            <div class="fixlist">
              <div v-for="(br, i) in bonusRows" :key="i" class="fixrow">
                <span class="fn">{{ br.description }}</span>
                <b class="ok-text">{{ fmt(br.amount) }}</b>
              </div>
              <div v-if="bolsaIncome > 0" class="fixrow">
                <span class="fn">Bolsa / rendimentos</span>
                <b class="ok-text">{{ fmt(bolsaIncome) }}</b>
              </div>
              <div class="fixrow tot">
                <span class="fn">Total bônus + extra líq.</span>
                <b class="ok-text">{{ fmt(bonusLiq) }}</b>
              </div>
            </div>
          </div>
        </div>
      </section>

      <!-- Graficos: categoria + maiores -->
      <section>
        <h2>Gráficos</h2>
        <div class="grid2">
          <div class="card">
            <h3>Mapa de gastos (treemap)</h3>
            <p class="cap">Cada retângulo é uma categoria; a área é proporcional ao valor. Passe o mouse para o total.</p>
            <CategoryTreemap :items="treemapItems" height="340px" />
          </div>

          <div class="card">
            <h3>Gasto por categoria (casa completa)</h3>
            <p class="cap">Cartão + fixos. Barras <span class="amber-text">âmbar</span> = categoria com contas fixas. <b>Clique para ver as despesas.</b></p>
            <div class="bars">
              <div v-for="c in categories" :key="c.name" class="cat-item" :class="{ open: expandedCat === c.name }">
                <div class="bar-row cat-click" @click="toggleCatDrill(c.name)" role="button" tabindex="0" @keyup.enter="toggleCatDrill(c.name)">
                  <div class="name" :title="c.name"><span class="caret">▶</span>{{ c.name }}<span v-if="fixoCategories.has(c.name)" class="fx"> ·fixo</span></div>
                  <div class="bar-track" :title="`${c.name}: ${fmt(c.net_total)} — ${c.transaction_count} lanç.`">
                    <div class="bar-fill" :class="{ amber: fixoCategories.has(c.name) }" :style="{ width: pctOf(num(c.net_total), catMax) + '%' }" />
                    <span v-if="pctOf(num(c.net_total), catMax) > 34" class="bar-val inside" :style="{ right: `calc(${100 - pctOf(num(c.net_total), catMax)}% + 8px)` }">{{ fmt(c.net_total) }} · {{ c.percentage.toFixed(0) }}%</span>
                    <span v-else class="bar-val outside" :style="{ left: `calc(${pctOf(num(c.net_total), catMax)}% + 8px)` }">{{ fmt(c.net_total) }}</span>
                  </div>
                </div>
                <div v-if="expandedCat === c.name" class="drill">
                  <div class="drill-head"><span class="t">{{ c.name }}</span><span class="meta">{{ drillItems.length }} lançamento{{ drillItems.length === 1 ? "" : "s" }}</span></div>
                  <div v-for="(it, idx) in drillItems" :key="idx" class="drow">
                    <span class="dt">{{ it.date }}</span>
                    <span class="ds">{{ it.desc }}<span class="src" :class="it.source">{{ SRC_LABEL[it.source] }}</span><span v-if="it.reversal" class="src rev">estorno</span></span>
                    <span class="da" :class="{ neg: it.amount < 0 }">{{ fmt(it.amount) }}</span>
                  </div>
                  <p v-if="!drillItems.length" class="drill-empty">Sem lançamentos detalhados nesta categoria.</p>
                  <div v-else class="drow tot"><span class="dt"></span><span class="ds">Total</span><span class="da">{{ fmt(drillTotal) }}</span></div>
                </div>
              </div>
            </div>
          </div>

          <div class="card">
            <h3>Maiores lançamentos (cartão)</h3>
            <p class="cap">As 5 maiores compras da fatura.</p>
            <div v-if="topTransactions.length" class="bars">
              <div v-for="t in topTransactions" :key="t.id" class="bar-row">
                <div class="name" :title="t.description">{{ t.description }}</div>
                <div class="bar-track" :title="`${t.description}: ${fmt(t.amount)}`">
                  <div class="bar-fill" :style="{ width: pctOf(num(t.amount), topMax) + '%' }" />
                  <span v-if="pctOf(num(t.amount), topMax) > 34" class="bar-val inside" :style="{ right: `calc(${100 - pctOf(num(t.amount), topMax)}% + 8px)` }">{{ fmt(t.amount) }}</span>
                  <span v-else class="bar-val outside" :style="{ left: `calc(${pctOf(num(t.amount), topMax)}% + 8px)` }">{{ fmt(t.amount) }}</span>
                </div>
              </div>
            </div>
            <p v-else class="cap">Sem lançamentos de cartão.</p>
          </div>
        </div>
      </section>

      <!-- Dia da semana + parcelas -->
      <section v-if="weekTotal > 0 || installments.length">
        <div class="grid2">
          <div class="card" v-if="weekTotal > 0">
            <h3>Gasto por dia da semana (cartão)</h3>
            <p class="cap">Fim de semana concentra <b>{{ weekendPct.toFixed(0) }}%</b> do gasto de cartão da semana.</p>
            <div class="dow">
              <div v-for="(v, i) in weekday" :key="i" class="dow-col">
                <div class="dow-v">{{ kBRL(v) }}</div>
                <div class="dow-bar" :class="{ peak: v === weekdayMax && v > 0 }" :style="{ height: Math.max((v / weekdayMax) * 150, 4) + 'px' }" :title="`${WD_LABELS[i]}: ${fmt(v)}`" />
                <div class="dow-lbl">{{ WD_LABELS[i] }}</div>
              </div>
            </div>
          </div>

          <div class="card" v-if="installments.length">
            <h3>Carga de parcelamento</h3>
            <p class="cap">Parcelas ativas neste mês. O que já está travado para os próximos meses.</p>
            <div class="bars">
              <div v-for="(it, i) in installments" :key="i" class="bar-row">
                <div class="name" :title="it.description">{{ it.description }} <span class="inst-tag">{{ it.current }}/{{ it.total }}</span></div>
                <div class="bar-track" :title="`${it.description}: ${fmt(it.amount)}`">
                  <div class="bar-fill amber" :style="{ width: pctOf(num(it.amount), instMax) + '%' }" />
                  <span v-if="pctOf(num(it.amount), instMax) > 40" class="bar-val inside" :style="{ right: `calc(${100 - pctOf(num(it.amount), instMax)}% + 8px)` }">{{ fmt(it.amount) }}</span>
                  <span v-else class="bar-val outside" :style="{ left: `calc(${pctOf(num(it.amount), instMax)}% + 8px)` }">{{ fmt(it.amount) }}</span>
                </div>
              </div>
            </div>
            <div class="inst-foot">
              <div><span>Parcelas neste mês</span><b>{{ fmt(monthParcelas) }}</b></div>
              <div><span>Restante já contratado</span><b class="amber-text">{{ fmt(futureParcelas) }}</b></div>
            </div>
          </div>
        </div>
      </section>

      <!-- Sugestões de economia -->
      <section v-if="suggestions.length">
        <h2>Sugestões de economia</h2>
        <div class="saves">
          <div v-for="(s, i) in suggestions" :key="i" :class="['save', 'pri-' + s.pri]">
            <div class="st"><h4>{{ s.title }}</h4><span class="tag">{{ s.tag }}</span></div>
            <p>{{ s.body }}</p>
            <div class="impact">{{ s.impact }}</div>
          </div>
        </div>
        <div v-if="hasPotential" class="card total-save">
          <div class="ts-head">
            <h3>Potencial total de economia</h3>
            <b class="ts-val">{{ fmt0(potentialMin) }} – {{ fmt0(potentialMax) }} / mês</b>
          </div>
          <p>Estimativa somando as alavancas acima. Concentrado nos fixos anômalos (água + energia) e no controle de impulso e assinaturas do cartão.</p>
        </div>
      </section>

      <div class="foot-note">
        Valores líquidos (débitos − estornos). Fixos e receitas vêm de <strong>Receitas &amp; Fixos</strong> e não passam pelo cartão. Sugestões geradas automaticamente a partir dos seus dados.
      </div>
    </template>

    <!-- Empty -->
    <div v-else-if="!store.loading && !store.hasData" class="empty">
      <div class="empty-icon">📂</div>
      <h2 class="empty-h">Comece a acompanhar suas finanças</h2>
      <p>Importe uma fatura BTG ou cadastre receitas e despesas fixas.</p>
      <p class="hint">Vá em <strong>Receitas &amp; Fixos</strong> para adicionar salário, aluguel, energia e mais.</p>
    </div>

    <!-- ── Report (print / PDF) ── -->
    <ReportOverlay v-if="reportOpen && d" :title="reportTitle" @close="reportOpen = false">
      <div class="sheet">
        <div class="sheet-head">
          <div class="logo">₣</div>
          <div>
            <div class="t">{{ isAllMonths ? `Relatório do período` : `Relatório de ${formatMonthFilter(store.monthFilter ?? "")}` }}</div>
            <div class="s">Contracheque SIGEPE · Cartão BTG (ref. {{ refLabel() }}) · fixos + avulsos</div>
          </div>
          <div class="right">gerado em {{ genDate }}<br>{{ d.invoice_count }} fatura(s)</div>
        </div>
        <div class="sheet-body">

          <div class="kpis">
            <div class="kpi"><div class="l">Receita total</div><div class="v pos">{{ fmt0(income) }}</div><div class="sub" v-if="bolsaIncome > 0">contracheque + extra {{ fmt0(bolsaIncome) }}</div></div>
            <div class="kpi"><div class="l">Despesa total</div><div class="v">{{ fmt0(expense) }}</div><div class="sub">cartão + fixos + avulsos + descontos</div></div>
            <div class="kpi"><div class="l">Saldo {{ scopeWord }}</div><div class="v" :class="balancePositive ? 'pos' : 'neg'">{{ balancePositive ? "" : "− " }}{{ fmt0(Math.abs(balance)) }}</div><div class="sub">{{ balancePositive ? "sobra" : "déficit" }}</div></div>
            <div class="kpi" v-if="hasPayslip"><div class="l">Líquido contracheque</div><div class="v">{{ fmt0(salaryNet) }}</div><div class="sub">salário {{ fmt0(salaryLiqMonth) }} + bônus {{ fmt0(bonusPayslip) }}</div></div>
          </div>

          <div>
            <h3>Composição da despesa</h3>
            <p class="cap">Onde o dinheiro foi. Avulsos contam separado dos fixos.</p>
            <div class="compbar">
              <div v-if="cardNet > 0" class="seg card" :style="{ flexGrow: cardNet }" :title="`Cartão ${fmt(cardNet)}`"><span v-if="cardPct > 10">Cartão {{ cardPct.toFixed(1) }}%</span></div>
              <div v-if="fixo > 0" class="seg fix" :style="{ flexGrow: fixo }" :title="`Fixos ${fmt(fixo)}`"><span v-if="fixoPct > 10">Fixos {{ fixoPct.toFixed(1) }}%</span></div>
              <div v-if="variableExpense > 0" class="seg avul" :style="{ flexGrow: variableExpense }" :title="`Avulsos ${fmt(variableExpense)}`"><span v-if="avulsoPct > 10">Avulsos {{ avulsoPct.toFixed(1) }}%</span></div>
              <div v-if="payrollDed > 0" class="seg ded" :style="{ flexGrow: payrollDed }" :title="`Descontos ${fmt(payrollDed)}`"><span v-if="payrollPct > 10">Descontos {{ payrollPct.toFixed(1) }}%</span></div>
            </div>
            <div class="legend">
              <span v-if="cardNet > 0"><i class="dot card"></i> Cartão — {{ fmt(cardNet) }}</span>
              <span v-if="fixo > 0"><i class="dot fix"></i> Fixos — {{ fmt(fixo) }}</span>
              <span v-if="variableExpense > 0"><i class="dot avul"></i> Avulsos — {{ fmt(variableExpense) }}</span>
              <span v-if="payrollDed > 0"><i class="dot ded"></i> Descontos — {{ fmt(payrollDed) }}</span>
            </div>
          </div>

          <div class="cols" v-if="avulsoExpenses.length || deductionRows.length">
            <div class="panel hi-avul" v-if="avulsoExpenses.length">
              <h3>Despesas avulsas</h3>
              <p class="cap">Não-recorrentes — fora das contas fixas.</p>
              <div v-for="e in avulsoExpenses" :key="e.id" class="row"><span class="n"><span class="badge avul">avulso</span>{{ e.description }}</span><b>{{ fmt(e.amount) }}</b></div>
              <div class="row tot"><span class="n">Total avulso</span><b>{{ fmt(variableExpense) }}</b></div>
            </div>
            <div class="panel" v-if="deductionRows.length">
              <h3>Descontos da folha</h3>
              <p class="cap">Deduções do contracheque.</p>
              <div v-for="r in deductionRows" :key="r.description" class="row"><span class="n"><span class="badge ded">folha</span>{{ r.description }}</span><b>{{ fmt(r.amount) }}</b></div>
              <div class="row tot"><span class="n">Total descontos</span><b>{{ fmt(payrollDed) }}</b></div>
            </div>
          </div>

          <div v-if="hasPayslip">
            <h3>Teto do cartão</h3>
            <p class="cap">Quanto o cartão poderia consumir sem furar o orçamento (renda − contas fixas).</p>
            <div class="ceil">
              <div class="sim">
                <div class="h">Com renda recorrente</div>
                <div class="v" :class="cardOverCeiling ? 'over' : 'ok'">{{ fmt0(tetoRecorrente) }}</div>
                <div class="f">cartão {{ fmt0(cardNet) }} <b :class="cardOverCeiling ? 'over' : 'ok'">{{ cardOverCeiling ? "→ estourou" : "→ dentro" }}</b></div>
              </div>
              <div class="sim">
                <div class="h">Só salário permanente</div>
                <div class="v">{{ fmt0(tetoSalario) }}</div>
                <div class="f">sem bônus/CD temporário</div>
              </div>
            </div>
          </div>

          <div v-if="treemapItems.length">
            <h3>Mapa de gastos</h3>
            <p class="cap">Área proporcional ao valor da categoria.</p>
            <CategoryTreemap :items="treemapItems" height="300px" />
          </div>

          <div v-if="topCats.length">
            <h3>Top categorias no cartão</h3>
            <p class="cap">Maiores gastos {{ scopeWord }}.</p>
            <div class="rank">
              <div v-for="c in topCats" :key="c.name" class="rk">
                <span class="n">{{ c.name }}</span>
                <span class="bar" :style="{ width: pctOf(num(c.net_total), num(topCats[0].net_total)) + '%' }"></span>
                <b>{{ fmt0(num(c.net_total)) }}</b>
              </div>
            </div>
          </div>

          <div class="insight" :class="{ warn: !balancePositive }">
            <b>Leitura {{ scopeWord }}:</b>
            <template v-if="!balancePositive"> déficit de {{ fmt0(Math.abs(balance)) }}.<template v-if="variableExpense > 0"> Avulsos somam {{ fmt0(variableExpense) }} — sem eles o mês fecharia em {{ fmt0(income - (expense - variableExpense)) }}.</template></template>
            <template v-else> sobra de {{ fmt0(balance) }}<template v-if="savingsRate"> ({{ savingsRate.toFixed(0) }}% da receita)</template>.</template>
            <template v-if="cardNet > 0"> Cartão representa {{ cardPct.toFixed(0) }}% da despesa.</template>
          </div>

        </div>
      </div>
    </ReportOverlay>
  </div>
</template>

<style scoped>
/* alias global tokens → local names (so dark mode flows through) */
.dash {
  --ink: var(--clr-text-primary);
  --ink-2: var(--clr-text-secondary);
  --ink-3: var(--clr-text-muted);
  --line: var(--clr-stroke);
  --surface: var(--clr-surface);
  --surface-2: var(--clr-surface-alt);
  --accent: var(--clr-accent);
  --accent-soft: var(--clr-accent-light);
  --amber: var(--clr-amber);
  --amber-soft: var(--clr-amber-soft);
  --red: var(--clr-negative);
  --red-soft: var(--clr-red-soft);
  --track: var(--clr-track);
  --radius: var(--radius-lg);
  --shadow: var(--shadow-sm);

  padding: 1.75rem 2rem 4rem;
  max-width: 1320px;
  margin: 0 auto;
  color: var(--ink);
  font-variant-numeric: tabular-nums;
}

/* Header */
.top { margin-bottom: 1.25rem; }
.top-row { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; }
.top-actions { display: flex; align-items: center; gap: 0.75rem; }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--accent); font-weight: 700; margin-bottom: 6px; }
h1 { font-size: clamp(22px, 3vw, 32px); line-height: 1.1; letter-spacing: -.02em; font-weight: 800; color: var(--ink); }
.period { font-size: 12px; font-weight: 600; color: var(--ink-2); background: var(--surface-2); padding: 3px 10px; border-radius: 100px; }
.month-select { font-family: inherit; font-size: 13px; font-weight: 600; color: var(--ink); background: var(--surface); border: 1px solid var(--line); border-radius: 8px; padding: 6px 10px; cursor: pointer; outline: none; }
.month-select:focus { border-color: var(--accent); }
.avg-toggle { display: inline-flex; border: 1px solid var(--line); border-radius: 8px; overflow: hidden; }
.avg-toggle button { font-family: inherit; font-size: 12.5px; font-weight: 600; color: var(--ink-2); background: var(--surface); border: none; padding: 6px 11px; cursor: pointer; }
.avg-toggle button:hover { background: var(--surface-2); }
.avg-toggle button.active { background: var(--accent); color: #fff; }
.qa-btn { font-family: inherit; font-size: 12.5px; font-weight: 700; color: var(--accent); background: var(--surface); border: 1px solid var(--accent); border-radius: 8px; padding: 6px 11px; cursor: pointer; white-space: nowrap; }
.qa-btn:hover { background: var(--accent-soft); }
.qa-card { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; background: var(--surface); border: 1px solid var(--line); border-radius: 10px; padding: 10px 12px; margin-bottom: 12px; box-shadow: var(--shadow-sm); }
.qa-kind { display: inline-flex; border: 1px solid var(--line); border-radius: 8px; overflow: hidden; }
.qa-kind button { font-family: inherit; font-size: 12.5px; font-weight: 600; color: var(--ink-2); background: var(--surface); border: none; padding: 6px 10px; cursor: pointer; }
.qa-kind button.active:first-child { background: var(--red); color: #fff; }
.qa-kind button.active:last-child { background: var(--accent); color: #fff; }
.qa-in { font-family: inherit; font-size: 13px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--ink); outline: none; }
.qa-in:focus { border-color: var(--accent); }
.qa-in.qa-amt { width: 100px; }
.qa-mes { font-size: 12px; color: var(--ink-3); }
.qa-add { font-family: inherit; font-size: 13px; font-weight: 700; padding: 6px 14px; border-radius: 8px; border: none; background: var(--accent); color: #fff; cursor: pointer; }
.qa-add:disabled { opacity: .5; }
.qa-err { font-size: 12px; color: var(--red); flex-basis: 100%; }

/* Password modal */
.pw-overlay { position: fixed; inset: 0; background: rgba(20,33,30,.45); display: flex; align-items: center; justify-content: center; z-index: 200; }
.pw-modal { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); box-shadow: var(--shadow); padding: 24px; width: min(420px, 92vw); }
.pw-modal h3 { font-size: 17px; font-weight: 800; color: var(--ink); margin-bottom: 6px; }
.pw-sub { font-size: 13px; color: var(--ink-2); margin-bottom: 16px; }
.pw-input { width: 100%; font-family: inherit; font-size: 14px; padding: 10px 12px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--ink); outline: none; }
.pw-input:focus { border-color: var(--accent); }
.pw-remember { display: flex; align-items: center; gap: 8px; margin-top: 12px; font-size: 13px; color: var(--ink-2); cursor: pointer; user-select: none; }
.pw-remember input { width: 15px; height: 15px; accent-color: var(--accent); cursor: pointer; }
.pw-err { margin-top: 10px; font-size: 12.5px; color: var(--red); }
.pw-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px; }
.pw-btn { font-family: inherit; font-size: 13px; font-weight: 700; padding: 8px 16px; border-radius: 8px; cursor: pointer; border: 1px solid transparent; }
.pw-btn.primary { background: var(--accent); color: #fff; }
.pw-btn.primary:disabled { opacity: .5; cursor: default; }
.pw-btn.ghost { background: var(--surface); border-color: var(--line); color: var(--ink-2); }
.sub { color: var(--ink-2); margin-top: 10px; max-width: 72ch; font-size: 14px; }
.sub b { color: var(--ink); font-weight: 700; }

section { margin-top: 2rem; }
h2 { font-size: 12px; letter-spacing: .10em; text-transform: uppercase; color: var(--ink-3); font-weight: 700; margin-bottom: 1rem; display: flex; align-items: center; gap: 10px; }
h2::after { content: ""; flex: 1; height: 1px; background: var(--line); }

/* KPIs */
.kpis { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; }
.kpi { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); padding: 16px 16px 14px; box-shadow: var(--shadow); }
.kpi .lbl { font-size: 12px; color: var(--ink-2); font-weight: 600; margin-bottom: 6px; }
.kpi .val { font-size: 23px; font-weight: 800; letter-spacing: -.02em; line-height: 1.05; }
.kpi .foot { font-size: 11.5px; color: var(--ink-3); margin-top: 6px; }
.flag { border-left: 3px solid var(--line); }
.flag-ok { border-left-color: var(--accent); }
.flag-amber { border-left-color: var(--amber); }
.flag-red { border-left-color: var(--red); }
.ok-text { color: var(--accent); }
.red-text { color: var(--red); }
.green-text { color: var(--accent); }
.amber-text { color: var(--amber); font-weight: 700; }
.pill { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; font-weight: 700; padding: 2px 8px; border-radius: 999px; margin-top: 8px; }
.pill.warn { background: var(--amber-soft); color: var(--amber); }
.pill.bad { background: var(--red-soft); color: var(--red); }

/* Cards + grid */
.grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
.card { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); padding: 18px; box-shadow: var(--shadow); }
.card h3 { font-size: 15px; font-weight: 700; letter-spacing: -.01em; margin-bottom: 3px; color: var(--ink); }
.card .cap { font-size: 12px; color: var(--ink-3); margin-bottom: 16px; }
.card .cap b { color: var(--ink-2); }

/* Composition split */
.split { display: flex; height: 40px; border-radius: 8px; overflow: hidden; gap: 2px; background: var(--track); }
.seg { display: flex; align-items: center; justify-content: center; color: #fff; font-size: 12px; font-weight: 700; min-width: 3px; white-space: nowrap; overflow: hidden; }
.seg-card { background: var(--accent); }
.seg-fixo { background: var(--amber); }
.seg-avulso { background: var(--violet, #8b5cf6); }
.seg-payroll { background: var(--red); }
.legend { display: flex; gap: 18px; margin-top: 12px; font-size: 12.5px; color: var(--ink-2); flex-wrap: wrap; }
.legend span { display: inline-flex; align-items: center; gap: 6px; }
.dot { width: 10px; height: 10px; border-radius: 3px; display: inline-block; }
.dot-card { background: var(--accent); }
.dot-fixo { background: var(--amber); }
.dot-avulso { background: var(--violet, #8b5cf6); }
.dot-payroll { background: var(--red); }
.note { margin-top: 16px; font-size: 12.5px; color: var(--ink-3); }
.note b { color: var(--ink-2); }

/* Fix list */
.fixlist { display: flex; flex-direction: column; }
.fixrow { display: flex; justify-content: space-between; align-items: center; padding: 10px 0; border-bottom: 1px solid var(--line); font-size: 14px; }
.fixrow:last-child { border-bottom: none; }
.fixrow .fn { color: var(--ink-2); display: flex; align-items: center; gap: 8px; }
.fixrow .fn.hot { color: var(--red); }
.fixrow b { font-weight: 700; color: var(--ink); }
.fixrow.tot { font-weight: 800; }
.fixrow.tot .fn, .fixrow.tot b { color: var(--ink); font-weight: 800; }
.badge-hot { font-size: 10px; font-weight: 800; text-transform: uppercase; padding: 1px 6px; border-radius: 999px; background: var(--red-soft); color: var(--red); }
.badge-kind { font-size: 10px; font-weight: 800; text-transform: uppercase; padding: 1px 6px; border-radius: 999px; }
.badge-kind.expense { background: var(--red-soft); color: var(--red); }
.badge-kind.income { background: rgba(16, 185, 129, .14); color: var(--accent); }
.av-cat { color: var(--ink-3, var(--ink-2)); font-size: 11.5px; font-weight: 600; margin-left: 4px; }
.avrow { position: relative; }
.avright { display: inline-flex; align-items: center; gap: 8px; }
.avactions { display: inline-flex; gap: 4px; margin-left: 4px; opacity: 0; transition: opacity .12s; }
.avrow:hover .avactions { opacity: 1; }
.av-ic { font-family: inherit; font-size: 12px; line-height: 1; padding: 4px 7px; border-radius: 6px; border: 1px solid var(--line); background: var(--surface, #fff); color: var(--ink-2); cursor: pointer; }
.av-ic:hover { background: var(--track); }
.av-ic.del:hover { background: var(--red-soft); color: var(--red); border-color: var(--red); }
.qa-cancel { font-family: inherit; font-size: 13px; font-weight: 700; padding: 6px 14px; border-radius: 8px; border: 1px solid var(--line); background: transparent; color: var(--ink-2); cursor: pointer; }
.badge-folha { font-size: 9.5px; font-weight: 700; text-transform: uppercase; padding: 1px 6px; border-radius: 999px; background: var(--accent-soft); color: var(--accent); margin-left: 4px; }

/* Bars */
.bars { display: flex; flex-direction: column; gap: 10px; }
.bar-row { display: grid; grid-template-columns: 130px 1fr; gap: 12px; align-items: center; }
.bar-row .name { font-size: 13px; color: var(--ink-2); text-align: right; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.bar-row .name .fx { color: var(--amber); font-weight: 700; }
.inst-tag { color: var(--ink-3); font-weight: 600; }
.bar-track { position: relative; background: var(--track); border-radius: 5px; height: 26px; }
.bar-fill { position: absolute; left: 0; top: 0; bottom: 0; background: var(--accent); border-radius: 5px; min-width: 3px; }
.bar-fill.amber { background: var(--amber); }
.bar-val { position: absolute; top: 50%; transform: translateY(-50%); font-size: 12px; font-weight: 700; white-space: nowrap; }
.bar-val.inside { color: #fff; }
.bar-val.outside { color: var(--ink-2); }

/* Category drill-down */
.cat-item { border-radius: 8px; }
.cat-item.open { background: var(--track); padding: 8px; margin: -8px -8px 0; }
.cat-click { cursor: pointer; border-radius: 6px; }
.cat-click:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.bar-row .name .caret { display: inline-block; font-size: 9px; color: var(--ink-3); margin-right: 5px; transition: transform .15s; }
.cat-item.open .caret { transform: rotate(90deg); color: var(--accent); }
.drill { margin-top: 10px; border: 1px solid var(--line); border-radius: 10px; overflow: hidden; background: var(--surface); }
.drill-head { display: flex; align-items: center; gap: 10px; padding: 9px 13px; background: var(--track); border-bottom: 1px solid var(--line); }
.drill-head .t { font-weight: 800; font-size: 13px; }
.drill-head .meta { margin-left: auto; font-size: 11.5px; color: var(--ink-3); }
.drow { display: grid; grid-template-columns: auto 1fr auto; gap: 10px; align-items: center; padding: 8px 13px; border-bottom: 1px solid var(--line-2, var(--line)); font-size: 13px; }
.drow:last-child { border-bottom: none; }
.drow .dt { color: var(--ink-3); font-size: 11.5px; white-space: nowrap; }
.drow .ds { color: var(--ink); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.drow .da { font-weight: 700; text-align: right; white-space: nowrap; }
.drow .da.neg { color: var(--red); }
.drow.tot { background: var(--track); font-weight: 800; }
.drow.tot .ds { color: var(--ink); }
.src { font-size: 9.5px; font-weight: 800; text-transform: uppercase; padding: 1px 6px; border-radius: 999px; margin-left: 7px; }
.src.card { background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--accent); }
.src.fix { background: color-mix(in srgb, var(--amber) 18%, transparent); color: var(--amber); }
.src.avul { background: color-mix(in srgb, #8b5cf6 20%, transparent); color: #8b5cf6; }
.src.folha, .src.rev { background: color-mix(in srgb, var(--red) 16%, transparent); color: var(--red); }
.drill-empty { font-size: 12.5px; color: var(--ink-3); padding: 10px 13px; margin: 0; }

/* Weekday columns */
.dow { display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; align-items: end; height: 200px; margin-top: 4px; }
.dow-col { display: flex; flex-direction: column; align-items: center; justify-content: flex-end; gap: 8px; height: 100%; }
.dow-v { font-size: 11px; color: var(--ink-2); font-weight: 700; }
.dow-bar { width: 100%; background: var(--accent); border-radius: 5px 5px 3px 3px; min-height: 4px; }
.dow-bar.peak { background: var(--amber); }
.dow-lbl { font-size: 12px; color: var(--ink-3); font-weight: 600; }

/* Installment footer */
.inst-foot { margin-top: 16px; padding-top: 14px; border-top: 1px solid var(--line); display: flex; flex-direction: column; gap: 8px; }
.inst-foot div { display: flex; justify-content: space-between; font-size: 13px; }
.inst-foot span { color: var(--ink-2); }
.inst-foot b { font-weight: 700; }

/* Savings */
.saves { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
.save { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); padding: 18px 18px 16px; box-shadow: var(--shadow); border-top: 3px solid var(--accent); }
.save.pri-red { border-top-color: var(--red); }
.save.pri-amber { border-top-color: var(--amber); }
.save.pri-accent { border-top-color: var(--accent); }
.save .st { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 8px; }
.save h4 { font-size: 15px; font-weight: 700; color: var(--ink); }
.save .tag { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: .05em; color: var(--ink-3); white-space: nowrap; }
.save p { margin-bottom: 8px; font-size: 13.5px; color: var(--ink-2); }
.save .impact { font-size: 13px; font-weight: 700; color: var(--accent); }
.save.pri-red .impact { color: var(--red); }
.save.pri-amber .impact { color: var(--amber); }
.total-save { margin-top: 14px; border-top: 3px solid var(--accent); }
.ts-head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; }
.ts-head h3 { margin: 0; }
.ts-val { font-size: 20px; font-weight: 800; color: var(--accent); white-space: nowrap; }
.total-save p { margin-top: 8px; font-size: 13px; color: var(--ink-2); }

/* Filter / messages */
.filter-badge { display: flex; align-items: center; justify-content: space-between; padding: 0.5rem 1rem; background: var(--accent-soft); border: 1px solid var(--accent); border-radius: 8px; font-size: 13px; color: var(--accent); margin-top: 1rem; }
.filter-badge strong { font-weight: 700; }
.clear-filter { background: none; border: 1px solid var(--accent); border-radius: 5px; color: var(--accent); cursor: pointer; font-size: 12px; font-weight: 600; padding: 2px 9px; }
.msg-error { display: flex; align-items: center; gap: 8px; padding: 10px 14px; border-radius: 8px; font-size: 13px; margin-top: 1rem; background: var(--red-soft); color: var(--red); border: 1px solid var(--red); }

/* Loading */
.loading { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-top: 1.5rem; }
.shimmer { height: 88px; border-radius: var(--radius); background: linear-gradient(90deg, var(--surface-2) 25%, var(--track) 50%, var(--surface-2) 75%); background-size: 200% 100%; animation: sh 1.4s infinite; }
@keyframes sh { 0% { background-position: 200% 0; } 100% { background-position: -200% 0; } }

/* Footer + empty */
.foot-note { margin-top: 2rem; font-size: 12px; color: var(--ink-3); border-top: 1px solid var(--line); padding-top: 14px; }
.empty { text-align: center; padding: 5rem 2rem; color: var(--ink-2); }
.empty-icon { font-size: 3rem; margin-bottom: 1rem; }
.empty-h { font-size: 1.125rem; font-weight: 700; color: var(--ink); margin-bottom: 0.5rem; }
.empty-h::after { display: none; }
.empty .hint { font-size: 0.8125rem; color: var(--ink-3); margin-top: 0.4rem; }

@media (max-width: 900px) {
  .kpis { grid-template-columns: 1fr 1fr; }
  .grid2, .saves { grid-template-columns: 1fr; }
  .loading { grid-template-columns: 1fr 1fr; }
  .bar-row { grid-template-columns: 100px 1fr; }
}
</style>
