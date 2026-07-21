<script setup lang="ts">
import { onMounted, ref, computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { importPayslip, savePayslip, listPayslips, removePayslip } from "@/services/tauri.service";
import type { Payslip, PayslipItem } from "@/types/api.types";

const list = ref<Payslip[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const busy = ref(false);
const expanded = ref<string | null>(null);

// confirm modal (batch)
const previews = ref<Payslip[]>([]);
const openMonth = ref<string | null>(null);

const n = (s: string) => parseFloat(s) || 0;
const brl = (v: number | string) => "R$ " + Math.round(typeof v === "string" ? n(v) : v).toLocaleString("pt-BR");
const brlF = (v: number | string) => (typeof v === "string" ? n(v) : v).toLocaleString("pt-BR", { style: "currency", currency: "BRL", minimumFractionDigits: 2 });
const MONTHS = ["Jan","Fev","Mar","Abr","Mai","Jun","Jul","Ago","Set","Out","Nov","Dez"];
const ml = (m: string) => { const [y, mo] = m.split("-"); return `${MONTHS[parseInt(mo) - 1] ?? mo}/${y.slice(2)}`; };

async function load(): Promise<void> {
  loading.value = true;
  try { list.value = await listPayslips(); }
  catch (e) { error.value = e instanceof Error ? e.message : String(e); }
  finally { loading.value = false; }
}
onMounted(load);

async function pickAndImport(): Promise<void> {
  error.value = null;
  const sel = await open({ multiple: true, filters: [{ name: "PDF", extensions: ["pdf"] }] });
  if (!sel) return;
  const paths = Array.isArray(sel) ? sel : [sel];
  if (!paths.length) return;
  busy.value = true;
  const errs: string[] = [];
  const parsed: Payslip[] = [];
  try {
    for (const path of paths) {
      try {
        parsed.push(await importPayslip(path));
      } catch (e) {
        errs.push(`${path.split("/").pop()}: ${e instanceof Error ? e.message : e}`);
      }
    }
    parsed.sort((a, b) => a.month.localeCompare(b.month));
    // Dedup by month within the batch (keep last parsed).
    const byMonth = new Map(parsed.map((p) => [p.month, p]));
    previews.value = [...byMonth.values()];
    openMonth.value = previews.value.length === 1 ? previews.value[0].month : null;
    if (errs.length) error.value = errs.join(" · ");
  } finally {
    busy.value = false;
  }
}

// Toggle a rendimento salário↔bônus; recompute that payslip's net split live.
function toggleClass(p: Payslip, it: PayslipItem): void {
  if (it.kind !== "rendimento" || it.offsetting) return;
  it.class = it.class === "salario" ? "bonus" : "salario";
  const salary = p.items
    .filter((i) => i.kind === "rendimento" && i.class === "salario" && !i.offsetting)
    .reduce((a, i) => a + n(i.net_share), 0);
  p.salary_liq = String(Math.round(salary * 100) / 100);
  p.bonus_liq = String(Math.round((n(p.net) - salary) * 100) / 100);
}

function removeFromBatch(month: string): void {
  previews.value = previews.value.filter((p) => p.month !== month);
}

async function confirmSave(): Promise<void> {
  if (!previews.value.length) return;
  busy.value = true;
  error.value = null;
  try {
    for (const p of previews.value) await savePayslip(p);
    previews.value = [];
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = false;
  }
}

async function remove(month: string): Promise<void> {
  if (!confirm(`Remover contracheque de ${ml(month)}?`)) return;
  await removePayslip(month);
  await load();
}

const hasData = computed(() => list.value.length > 0);
const avgNet = computed(() => hasData.value ? list.value.reduce((a, p) => a + n(p.net), 0) / list.value.length : 0);
const latest = computed(() => list.value[0] ?? null); // list is month DESC
const avgGross = computed(() => hasData.value ? list.value.reduce((a, p) => a + n(p.real_gross), 0) / list.value.length : 0);
const avgDed = computed(() => hasData.value ? list.value.reduce((a, p) => a + n(p.deductions), 0) / list.value.length : 0);
const netMax = computed(() => Math.max(1, ...list.value.map((p) => n(p.net))) * 1.12);
const chartMonths = computed(() => [...list.value].reverse()); // asc for the bars

function rend(p: Payslip) { return p.items.filter((i) => i.kind === "rendimento"); }
function desc(p: Payslip) { return p.items.filter((i) => i.kind === "desconto"); }
function cdNet(p: Payslip): number {
  return p.items.filter((i) => i.kind === "rendimento" && !i.offsetting && /CARGO DE DIRE/i.test(i.description))
    .reduce((a, i) => a + n(i.net_share), 0);
}
</script>

<template>
  <div class="page">
    <header class="top">
      <div>
        <p class="eyebrow">Contracheque · salário líquido</p>
        <h1>Meus contracheques</h1>
        <p class="sub">O <b>líquido</b> de cada mês alimenta a renda e o teto do cartão. CD e eventuais entram como <b>bônus</b>.</p>
      </div>
      <button class="btn" :disabled="busy" @click="pickAndImport">📄 Importar contracheques (PDF)</button>
    </header>

    <div v-if="error" class="msg err">⚠ {{ error }}</div>
    <div v-if="loading" class="state">Carregando…</div>
    <div v-else-if="!hasData" class="state">Nenhum contracheque importado. Clique em “Importar contracheque (PDF)”.</div>

    <template v-else>
      <div class="kpis">
        <div class="kpi"><span class="lbl">Líquido médio</span><span class="val pos">{{ brl(avgNet) }}</span><span class="sub2">{{ list.length }} meses</span></div>
        <div class="kpi"><span class="lbl">Último líquido</span><span class="val pos">{{ latest ? brl(latest.net) : "—" }}</span><span class="sub2">{{ latest ? ml(latest.month) : "" }}</span></div>
        <div class="kpi"><span class="lbl">Bruto médio (real)</span><span class="val">{{ brl(avgGross) }}</span><span class="sub2">sem adiantamentos</span></div>
        <div class="kpi"><span class="lbl">Descontos médios</span><span class="val amber">{{ brl(avgDed) }}</span><span class="sub2">FUNPRESP, GEAP, PSS, IR</span></div>
      </div>

      <div class="card">
        <h2>Líquido por mês</h2>
        <div class="trend">
          <div class="tcol" v-for="p in chartMonths" :key="p.month">
            <div class="tbar" :style="{ height: (n(p.net) / netMax * 100) + '%' }"><span class="tv">{{ (n(p.net)/1000).toFixed(1) }}k</span></div>
            <span class="tmth">{{ ml(p.month).split("/")[0] }}</span>
          </div>
        </div>
      </div>

      <div class="card">
        <h2>Contracheques importados</h2>
        <p class="hint">Clique para ver rendimentos e descontos. Reimportar o mesmo mês substitui.</p>
        <div class="plist">
          <div class="prow" v-for="p in list" :key="p.month">
            <div class="phead" @click="expanded = expanded === p.month ? null : p.month">
              <span class="pmes">{{ ml(p.month) }}</span>
              <span class="pcol hidec"><span class="k">Bruto</span><span class="v">{{ brl(p.real_gross) }}</span></span>
              <span class="pcol hidec" v-if="cdNet(p) > 0"><span class="k">Líq. do CD</span><span class="v amber">{{ brl(cdNet(p)) }}</span></span>
              <span class="pcol"><span class="k">Líquido</span><span class="v net">{{ brl(p.net) }}</span></span>
              <button class="del" title="Remover" @click.stop="remove(p.month)">✕</button>
              <span class="chev">{{ expanded === p.month ? "▴" : "▾" }}</span>
            </div>
            <div class="pbody" v-if="expanded === p.month">
              <div class="grid2">
                <div>
                  <h3 class="r">Rendimentos</h3>
                  <div class="li" v-for="(it, i) in rend(p)" :key="'r'+i" :class="{ wash: it.offsetting }">
                    <span class="d">{{ it.description }}
                      <span v-if="it.offsetting" class="chip wash">anula</span>
                      <span v-else-if="it.class === 'bonus'" class="chip bon">bônus</span>
                      <span v-else class="chip sal">salário</span>
                    </span>
                    <span class="a">{{ brlF(it.amount) }}<span v-if="!it.offsetting" class="ns">líq {{ brlF(it.net_share) }}</span></span>
                  </div>
                  <div class="li tot"><span>Bruto real</span><span>{{ brlF(p.real_gross) }}</span></div>
                </div>
                <div>
                  <h3 class="d">Descontos</h3>
                  <div class="li" v-for="(it, i) in desc(p)" :key="'d'+i" :class="{ wash: it.offsetting }">
                    <span class="d">{{ it.description }}<span v-if="it.offsetting" class="chip wash">anula</span></span>
                    <span class="a">{{ brlF(it.amount) }}</span>
                  </div>
                  <div class="li tot"><span>Descontos</span><span>{{ brlF(p.deductions) }}</span></div>
                </div>
              </div>
              <div class="splitrow">
                <span>Salário líq. <b class="pos">{{ brl(p.salary_liq) }}</b></span>
                <span>Bônus líq. <b class="amber">{{ brl(p.bonus_liq) }}</b></span>
                <span>Líquido a receber <b class="pos">{{ brlF(p.net) }}</b></span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- Confirm modal (batch) -->
    <div v-if="previews.length" class="overlay" @click.self="previews = []">
      <div class="modal wide">
        <h3>Conferir {{ previews.length }} contracheque{{ previews.length > 1 ? "s" : "" }}</h3>
        <p class="hint">Confira a classificação. Toque em salário/bônus para trocar. “anula” = adiantamento que se cancela.</p>
        <div class="pbatch">
          <div class="pcard" v-for="p in previews" :key="p.month">
            <div class="pcard-h" @click="openMonth = openMonth === p.month ? null : p.month">
              <span class="pmes">{{ ml(p.month) }}</span>
              <span class="pc-split">sal <b class="pos">{{ brl(p.salary_liq) }}</b> · bônus <b class="amber">{{ brl(p.bonus_liq) }}</b></span>
              <span class="pc-net">líq {{ brl(p.net) }}</span>
              <button class="del" title="Tirar do lote" @click.stop="removeFromBatch(p.month)">✕</button>
              <span class="chev">{{ openMonth === p.month ? "▴" : "▾" }}</span>
            </div>
            <div class="mitems" v-if="openMonth === p.month">
              <div class="mi" v-for="(it, i) in p.items.filter(x => x.kind === 'rendimento')" :key="i" :class="{ wash: it.offsetting }">
                <span class="d">{{ it.description }}</span>
                <span class="a">{{ brlF(it.amount) }}</span>
                <button v-if="it.offsetting" class="tag wash" disabled>anula</button>
                <button v-else class="tag" :class="it.class === 'bonus' ? 'bon' : 'sal'" @click="toggleClass(p, it)">
                  {{ it.class === "bonus" ? "bônus" : "salário" }}
                </button>
              </div>
            </div>
          </div>
        </div>
        <div class="macts">
          <button class="btn ghost" @click="previews = []">Cancelar</button>
          <button class="btn" :disabled="busy || !previews.length" @click="confirmSave">
            {{ busy ? "Salvando…" : `Salvar ${previews.length}` }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page { padding: 1.75rem 2rem 4rem; max-width: 1080px; margin: 0 auto; color: var(--clr-text-primary); font-variant-numeric: tabular-nums; }
.top { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; flex-wrap: wrap; margin-bottom: 1.3rem; }
.eyebrow { font-size: 11px; letter-spacing: .12em; text-transform: uppercase; color: var(--clr-accent); font-weight: 700; margin: 0 0 6px; }
h1 { font-size: 26px; font-weight: 800; letter-spacing: -.02em; margin: 0 0 6px; }
.sub { color: var(--clr-text-secondary); font-size: 13px; margin: 0; max-width: 70ch; }
.btn { font-family: inherit; font-size: .875rem; font-weight: 700; padding: .55rem 1.1rem; border-radius: 10px; border: none; background: var(--clr-accent); color: #fff; cursor: pointer; white-space: nowrap; }
.btn.ghost { background: transparent; color: var(--clr-text-secondary); border: 1px solid var(--clr-stroke); }
.btn:disabled { opacity: .6; cursor: default; }
.msg.err { background: var(--clr-red-soft); color: var(--clr-negative); border: 1px solid var(--clr-negative); padding: 10px 14px; border-radius: 8px; font-size: 13px; margin-bottom: 12px; }
.state { padding: 2rem 0; color: var(--clr-text-secondary); font-size: 14px; }

.kpis { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: .7rem; margin-bottom: 1rem; }
.kpi { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: 14px; padding: .95rem 1rem; box-shadow: var(--shadow-sm); display: flex; flex-direction: column; gap: .15rem; }
.kpi .lbl { font-size: 10.5px; font-weight: 700; letter-spacing: .05em; text-transform: uppercase; color: var(--clr-text-muted); }
.kpi .val { font-size: 1.4rem; font-weight: 780; letter-spacing: -.02em; }
.kpi .val.pos { color: var(--clr-positive); } .kpi .val.amber { color: var(--clr-amber); }
.kpi .sub2 { font-size: 11px; color: var(--clr-text-muted); }

.card { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: 16px; box-shadow: var(--shadow-sm); padding: 1.2rem 1.3rem; margin-bottom: 1rem; }
.card h2 { font-size: .95rem; font-weight: 700; margin: 0 0 .1rem; }
.hint { font-size: .78rem; color: var(--clr-text-muted); margin: 0 0 .9rem; }

.trend { display: flex; gap: 10px; align-items: flex-end; height: 150px; padding-top: 1rem; }
.tcol { flex: 1; display: flex; flex-direction: column; align-items: center; gap: 6px; height: 100%; justify-content: flex-end; }
.tbar { width: 66%; max-width: 46px; background: var(--clr-accent); border-radius: 6px 6px 0 0; position: relative; min-height: 2px; }
.tv { position: absolute; top: -18px; left: 50%; transform: translateX(-50%); font-size: 10.5px; font-weight: 700; white-space: nowrap; color: var(--clr-text-secondary); }
.tmth { font-size: 11px; color: var(--clr-text-muted); }

.plist { display: flex; flex-direction: column; gap: .5rem; }
.prow { border: 1px solid var(--clr-stroke); border-radius: 12px; overflow: hidden; }
.phead { display: grid; grid-template-columns: 90px 1fr auto auto 26px 20px; gap: 12px; align-items: center; padding: .7rem .9rem; cursor: pointer; }
.phead:hover { background: var(--clr-surface-alt); }
.pmes { font-weight: 700; font-size: .95rem; }
.pcol { text-align: right; font-size: 12px; }
.pcol .k { display: block; font-size: 9.5px; text-transform: uppercase; letter-spacing: .04em; color: var(--clr-text-muted); font-weight: 700; }
.pcol .v { font-weight: 700; } .pcol .v.net { color: var(--clr-positive); font-size: 14px; } .pcol .v.amber { color: var(--clr-amber); }
.del { background: transparent; border: none; color: var(--clr-negative); cursor: pointer; font-size: 13px; }
.chev { color: var(--clr-text-muted); text-align: center; }
.pbody { padding: 0 .9rem 1rem; border-top: 1px dashed var(--clr-stroke); }
.grid2 { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; margin-top: .9rem; }
.grid2 h3 { font-size: 11px; text-transform: uppercase; letter-spacing: .05em; margin: 0 0 .5rem; }
.grid2 h3.r { color: var(--clr-positive); } .grid2 h3.d { color: var(--clr-negative); }
.li { display: flex; justify-content: space-between; gap: 10px; font-size: 12.5px; padding: 4px 0; border-bottom: 1px solid var(--clr-stroke); }
.li:last-child { border-bottom: none; }
.li.wash { opacity: .55; }
.li .d { color: var(--clr-text-secondary); } .li .a { font-weight: 600; text-align: right; white-space: nowrap; }
.li .ns { display: block; font-size: 10.5px; color: var(--clr-text-muted); font-weight: 500; }
.li.tot { margin-top: 4px; border-top: 2px solid var(--clr-stroke); border-bottom: none; padding-top: 6px; font-weight: 800; }
.chip { font-size: 9.5px; font-weight: 700; border-radius: 100px; padding: 1px 7px; margin-left: 6px; }
.chip.sal { color: var(--clr-accent); background: var(--clr-accent-light); }
.chip.bon { color: var(--clr-amber); background: var(--clr-amber-soft); }
.chip.wash { color: var(--clr-text-muted); background: var(--clr-track); }
.splitrow { display: flex; gap: 1.5rem; flex-wrap: wrap; margin-top: 1rem; padding-top: .8rem; border-top: 1px solid var(--clr-stroke); font-size: 12.5px; color: var(--clr-text-secondary); }
.pos { color: var(--clr-positive); } .amber { color: var(--clr-amber); }

.overlay { position: fixed; inset: 0; background: rgba(0,0,0,.45); display: flex; align-items: center; justify-content: center; z-index: 50; padding: 1rem; }
.modal { background: var(--clr-surface); border: 1px solid var(--clr-stroke); border-radius: 16px; box-shadow: var(--shadow-lg); padding: 1.4rem 1.5rem; width: min(560px, 96vw); max-height: 90vh; overflow: auto; }
.modal.wide { width: min(680px, 96vw); }
.modal h3 { font-size: 17px; font-weight: 800; margin: 0 0 4px; }
.pbatch { display: flex; flex-direction: column; gap: .5rem; margin: 1rem 0; }
.pcard { border: 1px solid var(--clr-stroke); border-radius: 10px; overflow: hidden; }
.pcard-h { display: grid; grid-template-columns: 80px 1fr auto 22px 18px; gap: 10px; align-items: center; padding: .6rem .8rem; cursor: pointer; }
.pcard-h:hover { background: var(--clr-surface-alt); }
.pc-split { font-size: 12px; color: var(--clr-text-secondary); }
.pc-net { font-size: 13px; font-weight: 700; color: var(--clr-positive); white-space: nowrap; }
.pcard .mitems { padding: 0 .8rem .8rem; margin: 0; border-top: 1px dashed var(--clr-stroke); }
.mitems { display: flex; flex-direction: column; gap: 4px; margin: 1rem 0; }
.mi { display: grid; grid-template-columns: 1fr auto 84px; gap: 10px; align-items: center; font-size: 13px; padding: 5px 0; border-bottom: 1px solid var(--clr-stroke); }
.mi.wash { opacity: .55; }
.mi .a { font-weight: 600; }
.tag { font-family: inherit; font-size: 11px; font-weight: 700; border-radius: 8px; padding: 3px 0; cursor: pointer; border: 1px solid transparent; }
.tag.sal { background: var(--clr-accent); color: #fff; } .tag.bon { background: var(--clr-amber); color: #fff; }
.tag.wash { background: var(--clr-track); color: var(--clr-text-muted); cursor: default; }
.msum { display: flex; gap: 1.2rem; flex-wrap: wrap; padding: .8rem 0; border-top: 1px solid var(--clr-stroke); font-size: 13px; color: var(--clr-text-secondary); }
.macts { display: flex; justify-content: flex-end; gap: .6rem; margin-top: .8rem; }

@media (max-width: 720px) { .grid2 { grid-template-columns: 1fr; } .phead { grid-template-columns: 70px 1fr auto 26px 20px; } .pcol.hidec { display: none; } }
</style>
