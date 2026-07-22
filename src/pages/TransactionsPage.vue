<script setup lang="ts">
import { ref, computed, watch, onMounted } from "vue";
import type { Transaction, ManualEntry, BankEntry, Payslip } from "@/types/api.types";
import {
  getAllTransactions,
  listManualEntries,
  listBankEntries,
  listPayslips,
} from "@/services/tauri.service";
import { useSettingsStore } from "@/stores/settings.store";

// ── A unified movement (despesa/receita) assembled from every source ──
type Origin = "cartao" | "fixo" | "avulso" | "folha" | "contracheque" | "extrato";
type Tipo = "receita" | "despesa";

interface Movement {
  id: string;
  date: string; // "YYYY-MM-DD" — for sorting
  dateLabel: string; // display, with year
  month: string; // "YYYY-MM" — grouping + filter
  description: string;
  installment: string; // "(3/6)" or ""
  category: string;
  origin: Origin;
  tipo: Tipo;
  amount: number; // absolute magnitude (always positive)
  hasOverride: boolean;
}

const ORIGIN_META: Record<Origin, { label: string; cls: string }> = {
  cartao: { label: "Cartão", cls: "org-cartao" },
  fixo: { label: "Fixo", cls: "org-fixo" },
  avulso: { label: "Avulso", cls: "org-avulso" },
  folha: { label: "Folha", cls: "org-folha" },
  contracheque: { label: "Contracheque", cls: "org-contracheque" },
  extrato: { label: "Extrato", cls: "org-extrato" },
};

const settingsStore = useSettingsStore();

// ── raw data sources ──
const cardTx = ref<Transaction[]>([]);
const manual = ref<ManualEntry[]>([]);
const bank = ref<BankEntry[]>([]);
const payslips = ref<Payslip[]>([]);

const loading = ref(false);
const error = ref<string | null>(null);

// ── filter / view state ──
const search = ref("");
const monthFilter = ref("");
const tipo = ref<"tudo" | Tipo>("tudo");
const categoryFilter = ref("");
const view = ref<"misto" | "separado">("misto");

// Tipo is ignored (and visually disabled) in the "Separado" view.
const effectiveTipo = computed<"tudo" | Tipo>(() =>
  view.value === "separado" ? "tudo" : tipo.value
);

const MONTH_FULL = ["Janeiro","Fevereiro","Março","Abril","Maio","Junho","Julho","Agosto","Setembro","Outubro","Novembro","Dezembro"];
const MONTH_ABBR = ["jan","fev","mar","abr","mai","jun","jul","ago","set","out","nov","dez"];

function monthLabel(ym: string): string {
  const [y, m] = ym.split("-");
  return `${MONTH_FULL[parseInt(m, 10) - 1] ?? m} ${y}`;
}
function fmtFullDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  return `${d}/${MONTH_ABBR[parseInt(m, 10) - 1] ?? m}/${y}`;
}
function fmtMonthDate(ym: string): string {
  const [y, m] = ym.split("-");
  return `${MONTH_ABBR[parseInt(m, 10) - 1] ?? m}/${y}`;
}
// BRL magnitude (no sign). parseFloat lives at assembly; here we only format numbers.
function fmtBRL(n: number): string {
  return "R$ " + Math.abs(n).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}
function signed(n: number): string {
  return (n >= 0 ? "+ " : "− ") + fmtBRL(n);
}
function originLabel(o: Origin): string { return ORIGIN_META[o].label; }
function originCls(o: Origin): string { return ORIGIN_META[o].cls; }

// Mirror of the backend's payroll deduction → category mapping.
function dedCat(desc: string): string {
  const u = desc.toUpperCase();
  if (u.includes("IMPOSTO") || u.includes("IRRF") || u.includes("RENDA")) return "Impostos";
  if (u.includes("GEAP") || u.includes("SAUDE") || u.includes("SAÚDE") || u.includes("PSAUDE") || u.includes("PSAÚDE")) return "Saúde";
  if (u.includes("FUNPRESP") || u.includes("SEGURIDADE") || u.includes("PSS") || u.includes("PREVID")) return "Previdência";
  return "Descontos da folha";
}

async function load(): Promise<void> {
  loading.value = true;
  error.value = null;
  try {
    await settingsStore.loadConfig();
    cardTx.value = await getAllTransactions();
    // Optional sources — a missing/empty feature must not blank the page.
    try { manual.value = await listManualEntries(); } catch { manual.value = []; }
    try { bank.value = await listBankEntries(); } catch { bank.value = []; }
    try { payslips.value = await listPayslips(); } catch { payslips.value = []; }
  } catch (e) {
    error.value = String(e instanceof Error ? e.message : e);
  } finally {
    loading.value = false;
  }
}

onMounted(load);

// ── Assemble every source into one signed movement list ──
const movements = computed<Movement[]>(() => {
  const out: Movement[] = [];
  const overrides = settingsStore.config?.transaction_overrides ?? {};

  // Card transactions → despesa (origem Cartão). Reversals (estornos) come back as
  // a receita on the card so the saldo nets out correctly.
  for (const t of cardTx.value) {
    const amt = Math.abs(parseFloat(t.amount) || 0);
    if (amt === 0) continue;
    const isRev = t.is_reversal;
    out.push({
      id: `card-${t.id}`,
      date: t.date,
      dateLabel: fmtFullDate(t.date),
      month: t.date.slice(0, 7),
      description: isRev ? `Estorno · ${t.description}` : t.description,
      installment: t.installment ? `(${t.installment.current}/${t.installment.total})` : "",
      category: t.category,
      origin: "cartao",
      tipo: isRev ? "receita" : "despesa",
      amount: amt,
      hasOverride: !!overrides[t.id],
    });
  }

  // Manual entries → Fixo (recurring expense) / Avulso (one-off expense or extra income).
  for (const e of manual.value) {
    const amt = Math.abs(parseFloat(e.amount) || 0);
    if (amt === 0) continue;
    const isIncome = e.kind === "income";
    out.push({
      id: `man-${e.id}`,
      date: `${e.month}-01`,
      dateLabel: fmtMonthDate(e.month),
      month: e.month,
      description: e.description,
      installment: "",
      category: e.category,
      origin: isIncome ? "avulso" : e.recurring ? "fixo" : "avulso",
      tipo: isIncome ? "receita" : "despesa",
      amount: amt,
      hasOverride: false,
    });
  }

  // Bank statement entries → Extrato (income or expense by kind). Salary/fatura are
  // already excluded on import, so these don't double-count the payslip or the card.
  for (const b of bank.value) {
    const amt = Math.abs(parseFloat(b.amount) || 0);
    if (amt === 0) continue;
    const isIncome = b.kind === "income";
    out.push({
      id: `bank-${b.id}`,
      date: b.date,
      dateLabel: fmtFullDate(b.date),
      month: b.month || b.date.slice(0, 7),
      description: b.description,
      installment: "",
      category: b.category,
      origin: "extrato",
      tipo: isIncome ? "receita" : "despesa",
      amount: amt,
      hasOverride: false,
    });
  }

  // Payslips → each earning line is a receita (Contracheque), each non-offsetting
  // deduction is a despesa (Folha). Line-by-line so gross − deductions = líquido.
  for (const p of payslips.value) {
    for (const it of p.items) {
      if (it.offsetting) continue;
      const amt = Math.abs(parseFloat(it.amount) || 0);
      if (amt === 0) continue;
      if (it.kind === "rendimento") {
        out.push({
          id: `pay-r-${p.id}-${it.description}`,
          date: `${p.month}-01`,
          dateLabel: fmtMonthDate(p.month),
          month: p.month,
          description: it.description,
          installment: "",
          category: it.class === "bonus" ? "Renda Extra" : "Salário",
          origin: "contracheque",
          tipo: "receita",
          amount: amt,
          hasOverride: false,
        });
      } else if (it.kind === "desconto") {
        out.push({
          id: `pay-d-${p.id}-${it.description}`,
          date: `${p.month}-01`,
          dateLabel: fmtMonthDate(p.month),
          month: p.month,
          description: it.description,
          installment: "",
          category: dedCat(it.description),
          origin: "folha",
          tipo: "despesa",
          amount: amt,
          hasOverride: false,
        });
      }
    }
  }

  return out;
});

const hasAnyData = computed(() => movements.value.length > 0);

const availableMonths = computed(() => {
  const s = new Set(movements.value.map((m) => m.month));
  return Array.from(s).sort().reverse();
});

// Category chips reflect what is available for the current month + tipo scope.
const chipCategories = computed(() => {
  const s = new Set<string>();
  for (const m of movements.value) {
    if (monthFilter.value && m.month !== monthFilter.value) continue;
    if (effectiveTipo.value !== "tudo" && m.tipo !== effectiveTipo.value) continue;
    if (m.category) s.add(m.category);
  }
  return Array.from(s).sort((a, b) => a.localeCompare(b, "pt-BR"));
});

// Drop a category selection that no longer exists in the current scope.
watch(chipCategories, (cats) => {
  if (categoryFilter.value && !cats.includes(categoryFilter.value)) categoryFilter.value = "";
});

const filtered = computed<Movement[]>(() => {
  const q = search.value.toLowerCase().trim();
  return movements.value.filter((m) => {
    if (q && !m.description.toLowerCase().includes(q)) return false;
    if (monthFilter.value && m.month !== monthFilter.value) return false;
    if (effectiveTipo.value !== "tudo" && m.tipo !== effectiveTipo.value) return false;
    if (categoryFilter.value && m.category !== categoryFilter.value) return false;
    return true;
  });
});

// ── Summary KPIs (recompute from the filtered set) ──
const totalReceitas = computed(() =>
  filtered.value.filter((m) => m.tipo === "receita").reduce((s, m) => s + m.amount, 0)
);
const totalDespesas = computed(() =>
  filtered.value.filter((m) => m.tipo === "despesa").reduce((s, m) => s + m.amount, 0)
);
const countReceitas = computed(() => filtered.value.filter((m) => m.tipo === "receita").length);
const countDespesas = computed(() => filtered.value.filter((m) => m.tipo === "despesa").length);
const saldo = computed(() => totalReceitas.value - totalDespesas.value);
const count = computed(() => filtered.value.length);

// ── Misto view: chronological, grouped by month desc, per-month subtotals ──
interface MonthGroup {
  month: string;
  label: string;
  rows: Movement[];
  rec: number;
  desp: number;
  saldo: number;
}
const mistoGroups = computed<MonthGroup[]>(() => {
  const map = new Map<string, Movement[]>();
  for (const m of filtered.value) {
    if (!map.has(m.month)) map.set(m.month, []);
    map.get(m.month)!.push(m);
  }
  return [...map.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([month, rows]) => {
      const rec = rows.filter((r) => r.tipo === "receita").reduce((s, r) => s + r.amount, 0);
      const desp = rows.filter((r) => r.tipo === "despesa").reduce((s, r) => s + r.amount, 0);
      const sorted = [...rows].sort((a, b) => b.date.localeCompare(a.date));
      return { month, label: monthLabel(month), rows: sorted, rec, desp, saldo: rec - desp };
    });
});

// ── Separado view: per month, two sections (receitas / despesas) + month saldo ──
interface SepGroup {
  month: string;
  label: string;
  receitas: Movement[];
  despesas: Movement[];
  rec: number;
  desp: number;
  saldo: number;
}
const separadoGroups = computed<SepGroup[]>(() =>
  mistoGroups.value.map((g) => ({
    month: g.month,
    label: g.label,
    receitas: g.rows.filter((r) => r.tipo === "receita").sort((a, b) => b.amount - a.amount),
    despesas: g.rows.filter((r) => r.tipo === "despesa").sort((a, b) => b.amount - a.amount),
    rec: g.rec,
    desp: g.desp,
    saldo: g.saldo,
  }))
);

function pluralEntradas(n: number): string { return `${n} ${n === 1 ? "entrada" : "entradas"}`; }
function pluralSaidas(n: number): string { return `${n} ${n === 1 ? "saída" : "saídas"}`; }
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1 class="page-title">Despesas &amp; Receitas</h1>
      <span v-if="!loading" class="count-badge">{{ count }} {{ count === 1 ? "movimento" : "movimentos" }}</span>
    </div>
    <p class="subtitle">
      Todas as movimentações num só lugar — cartão, contas fixas, avulsos, descontos da folha e as
      receitas (salário, bolsa, renda extra e rendimentos).
    </p>

    <div v-if="loading" class="loading">Carregando…</div>
    <div v-else-if="error" class="error-msg">{{ error }}</div>

    <div v-else-if="!hasAnyData" class="empty-state">
      <div class="empty-icon">📂</div>
      <p>Nenhuma movimentação ainda. Importe uma fatura, um extrato ou um contracheque para começar.</p>
    </div>

    <template v-else>
      <!-- filters -->
      <div class="filters card">
        <label class="search">
          <span class="mag">🔍</span>
          <input v-model="search" type="text" placeholder="Buscar por descrição…" aria-label="Buscar" data-testid="tx-search" />
        </label>
        <select v-model="monthFilter" class="fld" aria-label="Mês" data-testid="tx-month-filter">
          <option value="">Todos os meses</option>
          <option v-for="m in availableMonths" :key="m" :value="m">{{ monthLabel(m) }}</option>
        </select>
        <div class="seg tipo" :class="{ disabled: view === 'separado' }" role="group" aria-label="Tipo de movimento">
          <button type="button" data-tipo="tudo" :class="{ on: tipo === 'tudo' }" @click="tipo = 'tudo'">Tudo</button>
          <button type="button" data-tipo="despesa" :class="{ on: tipo === 'despesa' }" @click="tipo = 'despesa'">↓ Despesas</button>
          <button type="button" data-tipo="receita" :class="{ on: tipo === 'receita' }" @click="tipo = 'receita'">↑ Receitas</button>
        </div>
      </div>

      <!-- category chip row -->
      <div class="chips">
        <button type="button" class="chip-f" :class="{ on: !categoryFilter }" @click="categoryFilter = ''">Todas</button>
        <button
          v-for="c in chipCategories"
          :key="c"
          type="button"
          class="chip-f"
          :class="{ on: categoryFilter === c }"
          @click="categoryFilter = c"
        >{{ c }}</button>
      </div>

      <!-- summary strip -->
      <div class="summary">
        <div class="kpi rec card">
          <div class="k-lbl">↑ Receitas</div>
          <div class="k-val">{{ fmtBRL(totalReceitas) }}</div>
          <div class="k-sub">{{ pluralEntradas(countReceitas) }}</div>
        </div>
        <div class="kpi desp card">
          <div class="k-lbl">↓ Despesas</div>
          <div class="k-val">{{ fmtBRL(totalDespesas) }}</div>
          <div class="k-sub">{{ pluralSaidas(countDespesas) }}</div>
        </div>
        <div class="kpi saldo card">
          <div class="k-lbl">Saldo</div>
          <div class="k-val" :class="saldo >= 0 ? 'pos' : 'neg'">{{ signed(saldo) }}</div>
          <div class="k-sub">receitas − despesas</div>
        </div>
        <div class="kpi count card">
          <div class="k-lbl">Movimentos</div>
          <div class="k-val">{{ count }}</div>
          <div class="k-sub">no período filtrado</div>
        </div>
      </div>

      <!-- list -->
      <div class="card list-card">
        <div class="list-head">
          <h2>Movimentos</h2>
          <span class="spacer"></span>
          <span class="seg-label">Exibição</span>
          <div class="seg view" role="group" aria-label="Modo de exibição">
            <button type="button" :class="{ on: view === 'misto' }" @click="view = 'misto'">Misto</button>
            <button type="button" :class="{ on: view === 'separado' }" @click="view = 'separado'">Separado</button>
          </div>
        </div>

        <!-- ===== MISTO ===== -->
        <div v-if="view === 'misto'">
          <div v-if="count > 0" class="table-scroll">
            <table>
              <thead>
                <tr>
                  <th></th>
                  <th>Data</th>
                  <th>Descrição</th>
                  <th>Categoria</th>
                  <th>Origem</th>
                  <th class="right">Valor</th>
                </tr>
              </thead>
              <tbody>
                <template v-for="g in mistoGroups" :key="g.month">
                  <tr class="grp">
                    <td colspan="6">
                      <div class="grp-inner">
                        <span class="grp-name">{{ g.label }}</span>
                        <span class="grp-tot">
                          <span class="g-rec">+ <b>{{ fmtBRL(g.rec) }}</b></span>
                          <span class="g-desp">− <b>{{ fmtBRL(g.desp) }}</b></span>
                          <span class="g-lbl">saldo</span>
                          <b class="g-sal" :class="{ neg: g.saldo < 0 }">{{ signed(g.saldo) }}</b>
                        </span>
                      </div>
                    </td>
                  </tr>
                  <tr v-for="m in g.rows" :key="m.id" class="mov" :class="m.tipo">
                    <td class="dir">{{ m.tipo === "receita" ? "↑" : "↓" }}</td>
                    <td class="date-cell">{{ m.dateLabel }}</td>
                    <td class="desc">{{ m.description }}<span v-if="m.installment" class="inst"> {{ m.installment }}</span></td>
                    <td>
                      <span class="cat-chip" :class="{ ovr: m.hasOverride }" :title="m.hasOverride ? 'Categoria ajustada manualmente' : undefined">
                        <span v-if="m.hasOverride" class="ov-dot">✎</span>{{ m.category }}
                      </span>
                    </td>
                    <td>
                      <span class="org" :class="originCls(m.origin)"><span class="odot"></span>{{ originLabel(m.origin) }}</span>
                    </td>
                    <td class="right">
                      <span class="val" :class="m.tipo">{{ m.tipo === "receita" ? "+ " : "− " }}{{ fmtBRL(m.amount) }}</span>
                    </td>
                  </tr>
                </template>
              </tbody>
            </table>
          </div>
          <div v-else class="empty">
            <span class="em-ic">🔎</span>
            Nenhum movimento para os filtros aplicados.
          </div>
        </div>

        <!-- ===== SEPARADO ===== -->
        <div v-else>
          <div v-if="count > 0">
            <div v-for="g in separadoGroups" :key="g.month" class="sep-view">
              <div class="sep-block">
                <div class="sep-head">
                  <span class="s-title rec">↑ Receitas</span>
                  <span class="s-count">{{ pluralEntradas(g.receitas.length) }} · {{ g.label }}</span>
                  <span class="s-sub rec">{{ signed(g.rec) }}</span>
                </div>
                <div v-if="g.receitas.length" class="table-scroll">
                  <table>
                    <tbody>
                      <tr v-for="m in g.receitas" :key="m.id" class="mov receita">
                        <td class="dir">↑</td>
                        <td class="date-cell">{{ m.dateLabel }}</td>
                        <td class="desc">{{ m.description }}<span v-if="m.installment" class="inst"> {{ m.installment }}</span></td>
                        <td><span class="cat-chip">{{ m.category }}</span></td>
                        <td><span class="org" :class="originCls(m.origin)"><span class="odot"></span>{{ originLabel(m.origin) }}</span></td>
                        <td class="right"><span class="val receita">+ {{ fmtBRL(m.amount) }}</span></td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <p v-else class="sep-empty">Sem receitas neste mês.</p>
              </div>

              <div class="sep-block">
                <div class="sep-head">
                  <span class="s-title desp">↓ Despesas</span>
                  <span class="s-count">{{ pluralSaidas(g.despesas.length) }} · {{ g.label }}</span>
                  <span class="s-sub desp">− {{ fmtBRL(g.desp) }}</span>
                </div>
                <div v-if="g.despesas.length" class="table-scroll">
                  <table>
                    <tbody>
                      <tr v-for="m in g.despesas" :key="m.id" class="mov despesa">
                        <td class="dir">↓</td>
                        <td class="date-cell">{{ m.dateLabel }}</td>
                        <td class="desc">{{ m.description }}<span v-if="m.installment" class="inst"> {{ m.installment }}</span></td>
                        <td>
                          <span class="cat-chip" :class="{ ovr: m.hasOverride }" :title="m.hasOverride ? 'Categoria ajustada manualmente' : undefined">
                            <span v-if="m.hasOverride" class="ov-dot">✎</span>{{ m.category }}
                          </span>
                        </td>
                        <td><span class="org" :class="originCls(m.origin)"><span class="odot"></span>{{ originLabel(m.origin) }}</span></td>
                        <td class="right"><span class="val despesa">− {{ fmtBRL(m.amount) }}</span></td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <p v-else class="sep-empty">Sem despesas neste mês.</p>
              </div>

              <div class="sep-saldo">
                <span class="ss-lbl">Saldo de {{ g.label }}</span>
                <span class="ss-val" :class="{ neg: g.saldo < 0 }">{{ signed(g.saldo) }}</span>
              </div>
            </div>
          </div>
          <div v-else class="empty">
            <span class="em-ic">🔎</span>
            Nenhum movimento para os filtros aplicados.
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.page {
  padding: 1.35rem 1.5rem 1.75rem;
  max-width: 1180px;
  margin: 0 auto;
  font-variant-numeric: tabular-nums;
}

.page-header { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; margin-bottom: 0.3rem; }
.page-title { font-size: 1.375rem; font-weight: 700; letter-spacing: -0.02em; color: var(--clr-text-primary); margin: 0; }
.count-badge { font-size: 0.72rem; color: var(--clr-text-muted); background: var(--clr-stroke-soft); padding: 2px 9px; border-radius: 100px; }
.subtitle { font-size: 0.8125rem; color: var(--clr-text-secondary); margin: 0 0 1.1rem; }

.card {
  background: var(--clr-surface);
  border: 1px solid var(--clr-stroke);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
}

/* ── Filters row ── */
.filters { display: flex; gap: 0.6rem; padding: 0.7rem 0.8rem; align-items: center; flex-wrap: wrap; margin-bottom: 0.8rem; }
.search {
  flex: 1 1 220px; display: flex; align-items: center; gap: 0.5rem;
  padding: 0.42rem 0.7rem; background: var(--clr-bg);
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-sm);
}
.search:focus-within { border-color: var(--clr-accent); }
.search .mag { color: var(--clr-text-muted); font-size: 0.85rem; }
.search input { flex: 1; border: none; outline: none; background: transparent; color: var(--clr-text-primary); font: inherit; font-size: 0.875rem; }
.search input::placeholder { color: var(--clr-text-muted); }

select.fld {
  padding: 0.42rem 0.7rem; font: inherit; font-size: 0.875rem; cursor: pointer;
  color: var(--clr-text-primary); background: var(--clr-bg);
  border: 1px solid var(--clr-stroke); border-radius: var(--radius-sm); outline: none;
}
select.fld:focus { border-color: var(--clr-accent); }

/* Segmented control (tipo + exibição) */
.seg { display: inline-flex; border: 1px solid var(--clr-stroke); border-radius: var(--radius-md); overflow: hidden; background: var(--clr-bg); }
.seg button {
  font: inherit; font-size: 0.8125rem; font-weight: 600; cursor: pointer;
  padding: 0.42rem 0.85rem; border: none; background: transparent; color: var(--clr-text-secondary);
  display: inline-flex; align-items: center; gap: 0.35rem; transition: background 0.1s, color 0.1s;
}
.seg button + button { border-left: 1px solid var(--clr-stroke); }
.seg button:hover { background: var(--clr-surface-alt); color: var(--clr-text-primary); }
.seg button.on { background: var(--clr-accent); color: #fff; }
.seg.tipo button.on[data-tipo="receita"] { background: var(--clr-positive); }
.seg.tipo button.on[data-tipo="despesa"] { background: var(--clr-negative); }
.seg.disabled { opacity: 0.4; pointer-events: none; }
.seg-label { font-size: 0.72rem; font-weight: 600; color: var(--clr-text-muted); text-transform: uppercase; letter-spacing: 0.04em; }

/* Category chip filter row */
.chips { display: flex; gap: 0.4rem; flex-wrap: wrap; margin-bottom: 1rem; }
.chip-f {
  font: inherit; font-size: 0.75rem; font-weight: 600; cursor: pointer;
  padding: 0.3rem 0.7rem; border-radius: 100px;
  border: 1px solid var(--clr-stroke); background: var(--clr-surface); color: var(--clr-text-secondary);
  transition: all 0.1s;
}
.chip-f:hover { background: var(--clr-surface-alt); color: var(--clr-text-primary); }
.chip-f.on { background: var(--clr-accent-light); border-color: var(--clr-accent); color: var(--clr-accent); }

/* ── Summary strip (KPIs) ── */
.summary { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0.8rem; margin-bottom: 1rem; }
.kpi { padding: 0.85rem 1rem; position: relative; overflow: hidden; }
.kpi .k-lbl { font-size: 0.68rem; font-weight: 700; letter-spacing: 0.05em; text-transform: uppercase; color: var(--clr-text-muted); display: flex; align-items: center; gap: 0.35rem; }
.kpi .k-val { font-size: 1.5rem; font-weight: 780; letter-spacing: -0.02em; margin-top: 0.25rem; }
.kpi .k-sub { font-size: 0.72rem; color: var(--clr-text-muted); margin-top: 0.1rem; }
.kpi.rec .k-val { color: var(--clr-positive); }
.kpi.desp .k-val { color: var(--clr-negative); }
.kpi.saldo .k-val.pos { color: var(--clr-positive); }
.kpi.saldo .k-val.neg { color: var(--clr-negative); }
.kpi::before { content: ""; position: absolute; left: 0; top: 0; bottom: 0; width: 3px; background: var(--clr-stroke); }
.kpi.rec::before { background: var(--clr-positive); }
.kpi.desp::before { background: var(--clr-negative); }
.kpi.saldo::before { background: var(--clr-accent); }
.kpi.count::before { background: var(--clr-text-muted); }

/* ── List card ── */
.list-card { overflow: hidden; }
.list-head { display: flex; align-items: center; gap: 0.75rem; padding: 0.75rem 1rem; border-bottom: 1px solid var(--clr-stroke); flex-wrap: wrap; }
.list-head h2 { font-size: 0.95rem; font-weight: 700; color: var(--clr-text-primary); margin: 0; }
.list-head .spacer { margin-left: auto; }

.table-scroll { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; font-size: 0.8125rem; min-width: 720px; }
thead th {
  text-align: left; padding: 0.5rem 0.75rem; border-bottom: 1px solid var(--clr-stroke);
  color: var(--clr-text-muted); font-size: 0.6875rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em;
  position: sticky; top: 0; background: var(--clr-surface);
}
thead th.right { text-align: right; }
tbody td { padding: 0.58rem 0.75rem; border-bottom: 1px solid var(--clr-stroke-soft); vertical-align: middle; color: var(--clr-text-primary); }
tbody tr:hover td { background: var(--clr-stroke-soft); }
.right { text-align: right; }

/* group header row */
tr.grp td { background: var(--clr-surface-alt); border-bottom: 1px solid var(--clr-stroke); padding: 0.5rem 0.75rem; }
.grp-inner { display: flex; align-items: center; gap: 0.75rem; flex-wrap: wrap; }
.grp-name { font-size: 0.8rem; font-weight: 800; letter-spacing: -0.01em; color: var(--clr-text-primary); }
.grp-tot { margin-left: auto; display: flex; align-items: center; gap: 1rem; font-size: 0.72rem; font-weight: 600; }
.grp-tot .g-rec { color: var(--clr-positive); }
.grp-tot .g-desp { color: var(--clr-negative); }
.grp-tot .g-sal { color: var(--clr-text-primary); }
.grp-tot .g-sal.neg { color: var(--clr-negative); }
.grp-tot .g-lbl { color: var(--clr-text-muted); font-weight: 600; }

/* direction indicator + income accent */
td.dir { width: 26px; text-align: center; color: var(--clr-text-muted); font-weight: 700; }
tr.mov.receita td.dir { color: var(--clr-positive); box-shadow: inset 3px 0 0 var(--clr-positive); }
tr.mov.receita:hover td { background: color-mix(in srgb, var(--clr-positive) 8%, var(--clr-surface)); }

.date-cell { color: var(--clr-text-secondary); white-space: nowrap; }
.desc { color: var(--clr-text-primary); font-weight: 500; }
.desc .inst { color: var(--clr-text-muted); font-weight: 400; }

.cat-chip {
  display: inline-flex; align-items: center; gap: 0.3rem; padding: 0.12rem 0.55rem;
  background: var(--clr-accent-light); color: var(--clr-accent);
  border-radius: 100px; font-size: 0.6875rem; font-weight: 600; white-space: nowrap;
}
.cat-chip.ovr { outline: 1px dashed color-mix(in srgb, var(--clr-accent) 55%, transparent); }
.cat-chip .ov-dot { font-size: 0.6rem; }

/* origem badge */
.org { display: inline-flex; align-items: center; gap: 0.35rem; font-size: 0.6875rem; font-weight: 600; color: var(--clr-text-secondary); white-space: nowrap; }
.org .odot { width: 7px; height: 7px; border-radius: 50%; background: var(--odot, var(--clr-text-muted)); flex: none; }
.org-cartao { --odot: var(--clr-chart-3); }
.org-fixo { --odot: var(--clr-chart-6); }
.org-avulso { --odot: var(--clr-amber); }
.org-folha { --odot: var(--clr-text-muted); }
.org-contracheque { --odot: var(--clr-positive); }
.org-extrato { --odot: var(--clr-chart-2); }

.val { font-weight: 700; white-space: nowrap; }
.val.receita { color: var(--clr-positive); }
.val.despesa { color: var(--clr-text-primary); }

/* separado view */
.sep-view { padding: 0.25rem 0 0.5rem; }
.sep-view + .sep-view { border-top: 1px solid var(--clr-stroke); }
.sep-block + .sep-block { border-top: 1px solid var(--clr-stroke); }
.sep-head { display: flex; align-items: center; gap: 0.6rem; padding: 0.7rem 1rem 0.3rem; flex-wrap: wrap; }
.sep-head .s-title { font-size: 0.82rem; font-weight: 800; }
.sep-head .s-title.rec { color: var(--clr-positive); }
.sep-head .s-title.desp { color: var(--clr-negative); }
.sep-head .s-count { font-size: 0.7rem; color: var(--clr-text-muted); background: var(--clr-stroke-soft); padding: 1px 8px; border-radius: 100px; }
.sep-head .s-sub { margin-left: auto; font-size: 0.95rem; font-weight: 780; }
.sep-head .s-sub.rec { color: var(--clr-positive); }
.sep-head .s-sub.desp { color: var(--clr-text-primary); }
.sep-empty { font-size: 0.8125rem; color: var(--clr-text-muted); padding: 0.4rem 1rem 0.8rem; }
.sep-saldo { display: flex; align-items: center; gap: 0.6rem; padding: 0.8rem 1rem; border-top: 1px solid var(--clr-stroke); background: var(--clr-surface-alt); }
.sep-saldo .ss-lbl { font-size: 0.72rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.04em; color: var(--clr-text-muted); }
.sep-saldo .ss-val { margin-left: auto; font-size: 1.15rem; font-weight: 800; color: var(--clr-positive); }
.sep-saldo .ss-val.neg { color: var(--clr-negative); }

/* states */
.empty { text-align: center; padding: 2.5rem 1rem; color: var(--clr-text-muted); font-size: 0.875rem; }
.empty .em-ic { font-size: 2rem; display: block; margin-bottom: 0.5rem; }
.empty-state { text-align: center; padding: 4rem 2rem; color: var(--clr-text-muted); }
.empty-icon { font-size: 2.5rem; margin-bottom: 0.75rem; }
.loading { text-align: center; padding: 3rem; color: var(--clr-text-muted); }
.error-msg { padding: 1rem; color: var(--clr-negative); background: var(--clr-red-soft); border-radius: var(--radius-lg); font-size: 0.875rem; }

/* ── Responsive ── */
@media (max-width: 860px) {
  .summary { grid-template-columns: repeat(2, 1fr); }
}
@media (max-width: 560px) {
  .page { padding: 1rem 0.85rem 1.3rem; }
  .summary { grid-template-columns: 1fr 1fr; }
  .kpi .k-val { font-size: 1.25rem; }
}
</style>
