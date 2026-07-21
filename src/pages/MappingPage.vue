<script setup lang="ts">
import { onMounted, ref, reactive, computed, watch } from "vue";
import { useInvoiceStore } from "@/stores/invoice.store";
import { useSettingsStore } from "@/stores/settings.store";

const store = useInvoiceStore();
const settings = useSettingsStore();

const num = (v?: string) => parseFloat(v ?? "0") || 0;
const okMsg = ref<string | null>(null);
const rowError = ref<string | null>(null);

const availableCategories = computed(() => settings.categoryGroups.map((g) => g.name));

const posTx = computed(() => store.allTransactions.filter((t) => !t.is_reversal));
const outrosList = computed(() =>
  posTx.value.filter((t) => t.category === "Outros").sort((a, b) => num(b.amount) - num(a.amount))
);
const outrosSum = computed(() => outrosList.value.reduce((a, t) => a + num(t.amount), 0));

// Group uncategorized transactions by merchant (same derived keyword), largest first.
// One row per merchant → categorizar uma vez aplica a todos os iguais.
interface OutroGroup { key: string; count: number; sum: number; sample: string; date: string; }
const outrosGroups = computed<OutroGroup[]>(() => {
  const m = new Map<string, OutroGroup>();
  for (const t of outrosList.value) { // outrosList already sorted by amount desc
    const k = merchantKey(t.description);
    if (!k) continue;
    if (!m.has(k)) m.set(k, { key: k, count: 0, sum: 0, sample: t.description, date: t.date });
    const g = m.get(k)!;
    g.count++;
    g.sum += num(t.amount);
    if (t.date > g.date) g.date = t.date; // most recent occurrence
  }
  return [...m.values()].sort((a, b) => b.sum - a.sum);
});

// per-merchant drafts: { [merchantKey]: { cat, kw } }
const drafts = reactive<Record<string, { cat: string; kw: string }>>({});
watch(
  outrosGroups,
  (groups) => {
    for (const g of groups) {
      if (!drafts[g.key]) drafts[g.key] = { cat: "", kw: g.key };
    }
  },
  { immediate: true }
);

// Must mirror the Rust `categorizer::normalize` so a derived keyword is always a
// substring of the normalized description — otherwise the tag matches nothing.
// Only strip installment suffixes and auth markers (`*`/`#`); keep digits and
// company suffixes, since removing mid-token pieces would break substring matching.
function deriveKeyword(desc: string): string {
  let s = desc.replace(/\(\d+\/\d+\)/g, " "); // installment (n/m)
  s = s.replace(/[*#]+/g, " "); // auth markers → space (same as backend)
  s = s.replace(/\s+/g, " ").trim().toUpperCase();
  return s || desc.trim().toUpperCase();
}

// merchant conflicts (same merchant across >1 category)
function merchantKey(desc: string): string {
  return deriveKeyword(desc);
}
interface Conflict { name: string; count: number; sum: number; cats: Record<string, number>; }
const conflicts = computed<Conflict[]>(() => {
  const m = new Map<string, Conflict>();
  for (const t of posTx.value) {
    const k = merchantKey(t.description);
    if (!k) continue;
    if (!m.has(k)) m.set(k, { name: k, count: 0, sum: 0, cats: {} });
    const c = m.get(k)!;
    c.count++;
    c.sum += num(t.amount);
    c.cats[t.category] = (c.cats[t.category] ?? 0) + 1;
  }
  return [...m.values()]
    .filter((c) => Object.keys(c.cats).length > 1)
    .sort((a, b) => b.sum - a.sum);
});

onMounted(async () => {
  await settings.loadConfig();
  await store.loadAllTransactions();
});

async function apply(g: OutroGroup): Promise<void> {
  rowError.value = null;
  okMsg.value = null;
  const dr = drafts[g.key];
  if (!dr || !dr.cat.trim()) { rowError.value = "Escolha uma categoria."; return; }
  const kw = (dr.kw || "").trim() || g.key;
  try {
    const changed = await store.mapKeyword(kw, dr.cat.trim());
    okMsg.value = `"${kw}" → ${dr.cat.trim()} · ${changed} lançamento(s) recategorizado(s).`;
    await settings.loadConfig();
  } catch (e) {
    rowError.value = String(e instanceof Error ? e.message : e);
  }
}

function fmt(v: number | string): string {
  const n = typeof v === "string" ? parseFloat(v) || 0 : v;
  return n.toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
}
function fmt0(v: number): string {
  return v.toLocaleString("pt-BR", { style: "currency", currency: "BRL", maximumFractionDigits: 0 });
}
const MONTHS = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
function shortDate(iso: string): string {
  const [y, m, d] = iso.split("-");
  return `${d}/${MONTHS[parseInt(m) - 1] ?? m}/${y}`;
}
</script>

<template>
  <div class="map">
    <header class="top">
      <p class="eyebrow">Categorização · Cartão BTG</p>
      <h1>Mapeamento de despesas</h1>
      <p class="sub">Categorizou um lançamento? O lojista <b>vira palavra-chave</b> da categoria e é aplicado a todos os iguais — agora e nas próximas faturas.</p>
    </header>

    <div v-if="okMsg" class="msg ok">✓ {{ okMsg }}</div>
    <div v-if="store.error || rowError" class="msg err">⚠ {{ rowError || store.error }}</div>

    <!-- stats -->
    <div class="stats">
      <div class="card stat flag-amber">
        <div class="l">Sem categoria ("Outros")</div>
        <div class="v">{{ outrosGroups.length }} <span class="u">lojistas · {{ outrosList.length }} lançamentos</span></div>
      </div>
      <div class="card stat flag-red">
        <div class="l">Valor invisível</div>
        <div class="v red">{{ fmt0(outrosSum) }}</div>
      </div>
      <div class="card stat">
        <div class="l">Conflitos de lojista</div>
        <div class="v">{{ conflicts.length }}</div>
      </div>
    </div>

    <!-- fila do outros -->
    <h2>Fila do "Outros" <span class="anno">maior valor primeiro · categorizar = vira palavra-chave</span></h2>
    <div class="card pad">
      <div v-if="!outrosList.length" class="empty-line">🎉 Tudo categorizado! Nenhum lançamento em "Outros".</div>
      <div v-for="g in outrosGroups" :key="g.key" class="row">
        <div class="info">
          <div class="desc">{{ g.sample }}</div>
          <div class="meta">
            <span class="badge-count">{{ g.count }}×</span>
            {{ g.count > 1 ? "lançamentos" : "lançamento" }} · até {{ shortDate(g.date) }}
          </div>
        </div>
        <div class="amt">{{ fmt(g.sum) }}</div>
        <div class="assign" v-if="drafts[g.key]">
          <input
            v-model="drafts[g.key].cat"
            list="cats"
            class="in cat"
            placeholder="Categoria…"
          />
          <span class="arrow">·</span>
          <input v-model="drafts[g.key].kw" class="in kw" title="Palavra-chave (editável)" />
          <button class="btn" :disabled="store.loading" @click="apply(g)">Aplicar</button>
        </div>
      </div>
      <datalist id="cats">
        <option v-for="c in availableCategories" :key="c" :value="c" />
      </datalist>
    </div>

    <!-- conflitos -->
    <h2 v-if="conflicts.length">Conflitos de lojista <span class="anno">mesmo lojista em categorias diferentes</span></h2>
    <div v-if="conflicts.length" class="card pad">
      <div class="conf-note">Estes lojistas caíram em categorias diferentes (overrides manuais divergentes). Categorize-os aqui ou na fila para unificar via palavra-chave — lançamentos com reclassificação manual mantêm a exceção.</div>
      <table>
        <thead><tr><th>Lojista</th><th>Lanç.</th><th>Categorias</th></tr></thead>
        <tbody>
          <tr v-for="c in conflicts" :key="c.name">
            <td class="m-name">{{ c.name }}</td>
            <td class="nowrap">{{ c.count }} · {{ fmt0(c.sum) }}</td>
            <td>
              <span v-for="(n, cat) in c.cats" :key="cat" class="cat-chip conflict">{{ cat }} ({{ n }})</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <div class="foot">
      Reaproveita <b>add_category_keyword</b> (adiciona keyword + recategoriza), <b>list_all_transactions</b> e as regras de categoria. Palavra-chave é editável antes de aplicar.
    </div>
  </div>
</template>

<style scoped>
.map {
  --ink: var(--clr-text-primary); --ink-2: var(--clr-text-secondary); --ink-3: var(--clr-text-muted);
  --line: var(--clr-stroke); --surface: var(--clr-surface); --surface-2: var(--clr-surface-alt);
  --accent: var(--clr-accent); --accent-soft: var(--clr-accent-light); --amber: var(--clr-amber);
  --red: var(--clr-negative); --red-soft: var(--clr-red-soft); --track: var(--clr-track);
  --radius: var(--radius-lg); --shadow: var(--shadow-sm);
  padding: 1.75rem 2rem 4rem; max-width: 1320px; margin: 0 auto; color: var(--ink); font-variant-numeric: tabular-nums;
}
.top { margin-bottom: 1.25rem; }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--accent); font-weight: 700; margin-bottom: 6px; }
h1 { font-size: 28px; font-weight: 800; letter-spacing: -.02em; margin-bottom: 8px; }
.sub { color: var(--ink-2); font-size: 14px; max-width: 72ch; }
.sub b { color: var(--ink); }

.msg { padding: 10px 14px; border-radius: 8px; font-size: 13px; margin-bottom: 12px; }
.msg.ok { background: var(--accent-soft); color: var(--accent); border: 1px solid var(--accent); }
.msg.err { background: var(--red-soft); color: var(--red); border: 1px solid var(--red); }

.stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-bottom: 1rem; }
.stat { padding: 16px; }
.stat .l { font-size: 12px; color: var(--ink-2); font-weight: 600; }
.stat .v { font-size: 24px; font-weight: 800; margin-top: 4px; }
.stat .v .u { font-size: 14px; color: var(--ink-2); font-weight: 600; }
.stat .v.red { color: var(--red); }
.flag-amber { border-left: 3px solid var(--amber); }
.flag-red { border-left: 3px solid var(--red); }

h2 { font-size: 12px; letter-spacing: .10em; text-transform: uppercase; color: var(--ink-3); font-weight: 700; margin: 2rem 0 1rem; display: flex; gap: 10px; align-items: center; }
h2::after { content: ""; flex: 1; height: 1px; background: var(--line); }
.anno { background: var(--accent-soft); color: var(--accent); font-size: 11px; font-weight: 700; padding: 1px 8px; border-radius: 100px; text-transform: none; letter-spacing: 0; }

.card { background: var(--surface); border: 1px solid var(--line); border-radius: var(--radius); box-shadow: var(--shadow); }
.pad { padding: 18px; }

.row { display: grid; grid-template-columns: 1fr auto auto; gap: 14px; align-items: center; padding: 11px 0; border-bottom: 1px solid var(--line); }
.row:last-child { border-bottom: none; }
.info { min-width: 0; }
.desc { font-size: 13.5px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.meta { font-size: 11.5px; color: var(--ink-3); margin-top: 2px; }
.badge-count { display: inline-block; font-weight: 700; color: var(--accent); background: var(--accent-soft); border-radius: 100px; padding: 0 7px; margin-right: 4px; }
.amt { font-size: 14px; font-weight: 700; white-space: nowrap; }
.assign { display: flex; align-items: center; gap: 8px; }
.in { font-family: inherit; font-size: 13px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--surface); color: var(--ink); outline: none; }
.in:focus { border-color: var(--accent); }
.in.cat { width: 150px; }
.in.kw { width: 150px; color: var(--ink-2); }
.arrow { color: var(--ink-3); }
.btn { font-family: inherit; font-size: 13px; font-weight: 700; padding: 6px 14px; border-radius: 8px; border: 1px solid var(--accent); background: var(--accent); color: #fff; cursor: pointer; }
.btn:disabled { opacity: .5; cursor: default; }
.empty-line { font-size: 14px; color: var(--ink-2); padding: 0.5rem 0; }

.conf-note { font-size: 12.5px; color: var(--ink-3); margin-bottom: 14px; }
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th { text-align: left; font-size: 11px; text-transform: uppercase; letter-spacing: .04em; color: var(--ink-3); font-weight: 700; padding: 0 8px 8px; border-bottom: 1px solid var(--line); }
td { padding: 11px 8px; border-bottom: 1px solid var(--line); vertical-align: middle; }
tr:last-child td { border-bottom: none; }
.m-name { font-weight: 600; }
.nowrap { white-space: nowrap; }
.cat-chip { display: inline-block; font-size: 11.5px; font-weight: 600; padding: 2px 9px; border-radius: 100px; margin: 1px 3px 1px 0; background: var(--accent-soft); color: var(--accent); }
.cat-chip.conflict { background: var(--red-soft); color: var(--red); }

.foot { margin-top: 2rem; font-size: 12px; color: var(--ink-3); border-top: 1px solid var(--line); padding-top: 14px; }
.foot b { color: var(--ink-2); }

@media (max-width: 820px) {
  .stats { grid-template-columns: 1fr; }
  .row { grid-template-columns: 1fr auto; }
  .assign { grid-column: 1 / -1; flex-wrap: wrap; }
  .in.cat, .in.kw { width: 100%; flex: 1; }
}
</style>
