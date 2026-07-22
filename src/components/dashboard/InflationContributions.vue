<script setup lang="ts">
// Dashboard breakdown of which categories contribute most to personal inflation.
import { computed, onMounted, ref } from "vue";
import { getPersonalInflationDetail } from "@/services/tauri.service";
import type { InflationContribution, PersonalInflationDetail } from "@/types/api.types";

const detail = ref<PersonalInflationDetail | null>(null);
const loaded = ref(false);

onMounted(async () => {
  try {
    detail.value = await getPersonalInflationDetail();
  } catch {
    // Sem índices em cache / sem gastos — mantém null (nada é renderizado).
  } finally {
    loaded.value = true;
  }
});

// ── Formatação ──
const fmt = (v: number, min = 1, max = 1) =>
  v.toLocaleString("pt-BR", { minimumFractionDigits: min, maximumFractionDigits: max });
const brl = (s?: string | null) =>
  (parseFloat(s ?? "0") || 0).toLocaleString("pt-BR", { style: "currency", currency: "BRL" });

// ── Sua inflação vs. oficial ──
const personalPct = computed(() => fmt(detail.value?.inflacao_pessoal_pct ?? 0, 1, 2) + "%");
const oficialPct = computed(() => fmt((detail.value?.inflacao_oficial ?? 0) * 100, 1, 2) + "%");
const diffPp = computed(() => Math.round((detail.value?.diferenca_pp ?? 0) * 10) / 10);
const diffTone = computed(() => (diffPp.value > 0 ? "bad" : diffPp.value < 0 ? "good" : "neutral"));
const diffSign = computed(() => (diffPp.value > 0 ? "+" : diffPp.value < 0 ? "−" : ""));
const diffAbs = computed(() => fmt(Math.abs(diffPp.value), 1, 1));
const ppLabel = computed(() => (Math.abs(diffPp.value) === 1 ? "ponto percentual" : "pontos percentuais"));
const diffText = computed(() =>
  diffPp.value > 0 ? "acima do IPCA oficial" : diffPp.value < 0 ? "abaixo do IPCA oficial" : "em linha com o IPCA oficial"
);

// ── Contribuições ──
const contribs = computed(() => detail.value?.contribuicoes ?? []);
const maxContrib = computed(() =>
  Math.max(0.0001, ...contribs.value.map((c) => Math.abs(c.contribuicao)))
);
const barPct = (c: InflationContribution) => Math.max(2, (Math.abs(c.contribuicao) / maxContrib.value) * 100);
const ppText = (c: InflationContribution) => {
  const sign = c.contribuicao < 0 ? "−" : "+";
  return `${sign}${fmt(Math.abs(c.contribuicao) * 100, 1, 2)} p.p.`;
};

// ── Impacto em reais ──
const hasPerda = computed(() => (parseFloat(detail.value?.perda_poder_compra ?? "0") || 0) > 0);
</script>

<template>
  <div v-if="detail" class="inflc">
    <div class="head">
      <h3>Sua inflação, em detalhe</h3>
      <p class="cap">Como cada categoria puxou a sua inflação e o que isso significa em reais.</p>
    </div>

    <!-- 1. Sua inflação vs. oficial -->
    <div class="cmp">
      <div class="cmp-k">
        <span class="l">Sua inflação</span>
        <span class="v" :class="diffTone">{{ personalPct }}</span>
      </div>
      <div class="cmp-k">
        <span class="l">IPCA oficial</span>
        <span class="v">{{ oficialPct }}</span>
      </div>
      <div class="cmp-diff" :class="diffTone">
        <span class="dv">{{ diffSign }}{{ diffAbs }}</span>
        <span class="dl">{{ ppLabel }} {{ diffText }}</span>
      </div>
    </div>

    <!-- 2. O que puxou -->
    <div v-if="contribs.length" class="sect">
      <h4>O que puxou sua inflação</h4>
      <p class="sub">Contribuição de cada categoria = peso do seu gasto × inflação da categoria.</p>
      <div class="bars">
        <div class="bar" v-for="c in contribs" :key="c.category">
          <div class="brow">
            <span class="nm">{{ c.category }}</span>
            <span class="pp" :class="{ neg: c.contribuicao < 0 }">{{ ppText(c) }}</span>
          </div>
          <div class="track">
            <div class="fill" :class="{ neg: c.contribuicao < 0 }" :style="{ width: barPct(c) + '%' }"></div>
          </div>
          <div class="meta">
            <span>peso {{ fmt(c.weight * 100, 1, 1) }}%</span>
            <span>inflação {{ fmt(c.inflacao * 100, 1, 2) }}%</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 3. Impacto em reais -->
    <div class="sect">
      <h4>Impacto em reais</h4>
      <div class="money">
        <div class="mc">
          <span class="l">Custo da sua cesta, atualizado</span>
          <span class="v">{{ brl(detail.custo_atualizado) }}</span>
          <span class="s">+{{ brl(detail.aumento_cesta) }} vs. hoje</span>
        </div>
        <div class="mc">
          <span class="l">Renda para manter o padrão</span>
          <span class="v">{{ brl(detail.renda_corrigida) }}</span>
          <span class="s">precisa de +{{ brl(detail.aumento_renda) }}</span>
          <span class="s2">conservador (só consumo): {{ brl(detail.renda_corrigida_consumo) }}</span>
        </div>
        <div class="mc" v-if="hasPerda">
          <span class="l">Perda de poder de compra</span>
          <span class="v neg">− {{ brl(detail.perda_poder_compra) }}</span>
        </div>
      </div>
    </div>

    <!-- 4. Simulação comportamental -->
    <div v-if="detail.impacto_comportamental_pct != null" class="sect">
      <h4>Simulação comportamental <span class="tag">simulação</span></h4>
      <p class="sub">Mantendo o mesmo padrão de consumo, o gasto tenderia a crescer:</p>
      <div class="simrow">
        <span class="sv">{{ fmt(detail.impacto_comportamental_pct, 1, 1) }}%</span>
        <span v-if="detail.consumo_adicional" class="sm">≈ {{ brl(detail.consumo_adicional) }} a mais</span>
      </div>
    </div>

    <!-- 5. Proveniências -->
    <ul v-if="detail.proveniencias.length" class="prov">
      <li v-for="(p, i) in detail.proveniencias" :key="i">{{ p }}</li>
    </ul>

    <!-- 6. Aviso -->
    <p v-if="detail.aviso" class="aviso">{{ detail.aviso }}</p>
  </div>

  <p v-else-if="loaded" class="empty">
    Atualize os índices do IPCA para ver o detalhamento da sua inflação.
  </p>
</template>

<style scoped>
.inflc { display: flex; flex-direction: column; gap: 16px; }
.head h3 { margin: 0; font-size: 14.5px; font-weight: 800; }
.cap { margin: 2px 0 0; font-size: 12px; color: var(--clr-text-muted); }
.empty { margin: 0; font-size: 12.5px; color: var(--clr-text-muted); }

/* 1. Sua inflação vs. oficial */
.cmp { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; align-items: stretch; }
.cmp-k, .cmp-diff { border: 1px solid var(--clr-stroke); border-radius: 12px; padding: 12px 14px; display: flex; flex-direction: column; gap: 3px; }
.cmp-k .l { font-size: 11.5px; font-weight: 600; color: var(--clr-text-secondary); }
.cmp-k .v { font-size: 24px; font-weight: 800; letter-spacing: -.02em; font-variant-numeric: tabular-nums; }
.cmp-k .v.bad { color: var(--clr-negative); }
.cmp-k .v.good { color: var(--clr-accent); }
.cmp-diff { justify-content: center; }
.cmp-diff .dv { font-size: 22px; font-weight: 800; letter-spacing: -.02em; font-variant-numeric: tabular-nums; }
.cmp-diff .dl { font-size: 11.5px; color: var(--clr-text-secondary); line-height: 1.4; }
.cmp-diff.bad { background: var(--clr-red-soft); }
.cmp-diff.bad .dv { color: var(--clr-negative); }
.cmp-diff.good { background: var(--clr-accent-light); }
.cmp-diff.good .dv { color: var(--clr-accent); }
.cmp-diff.neutral .dv { color: var(--clr-text-primary); }

/* sections */
.sect h4 { margin: 0 0 2px; font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: .03em; color: var(--clr-text-secondary); display: flex; align-items: center; gap: 8px; }
.sub { margin: 0 0 10px; font-size: 12px; color: var(--clr-text-muted); }

/* 2. Contribuições */
.bars { display: flex; flex-direction: column; gap: 11px; }
.bar { display: flex; flex-direction: column; gap: 3px; }
.brow { display: flex; justify-content: space-between; align-items: baseline; gap: 10px; font-size: 12.5px; }
.brow .nm { color: var(--clr-text-primary); font-weight: 600; }
.brow .pp { color: var(--clr-accent); font-weight: 800; font-variant-numeric: tabular-nums; }
.brow .pp.neg { color: var(--clr-amber); }
.track { height: 8px; border-radius: 5px; background: var(--clr-track); overflow: hidden; }
.fill { height: 100%; border-radius: 5px; background: var(--clr-accent); }
.fill.neg { background: var(--clr-amber); }
.meta { display: flex; gap: 14px; font-size: 11px; color: var(--clr-text-muted); font-variant-numeric: tabular-nums; }

/* 3. Impacto em reais */
.money { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 12px; }
.mc { border: 1px solid var(--clr-stroke); border-radius: 12px; padding: 12px 14px; display: flex; flex-direction: column; gap: 2px; }
.mc .l { font-size: 11.5px; font-weight: 600; color: var(--clr-text-secondary); }
.mc .v { font-size: 19px; font-weight: 800; letter-spacing: -.02em; font-variant-numeric: tabular-nums; }
.mc .v.neg { color: var(--clr-negative); }
.mc .s { font-size: 11.5px; color: var(--clr-text-muted); font-variant-numeric: tabular-nums; }
.mc .s2 { font-size: 11px; color: var(--clr-text-muted); font-variant-numeric: tabular-nums; margin-top: 2px; }

/* 4. Simulação */
.tag { font-size: 9.5px; font-weight: 800; letter-spacing: .04em; text-transform: uppercase; padding: 2px 7px; border-radius: 999px; background: var(--clr-amber-soft); color: var(--clr-amber); }
.simrow { display: flex; align-items: baseline; gap: 10px; flex-wrap: wrap; }
.simrow .sv { font-size: 21px; font-weight: 800; letter-spacing: -.02em; color: var(--clr-text-primary); font-variant-numeric: tabular-nums; }
.simrow .sm { font-size: 12.5px; color: var(--clr-text-secondary); font-variant-numeric: tabular-nums; }

/* 5. Proveniências */
.prov { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 4px; }
.prov li { font-size: 11.5px; color: var(--clr-text-muted); line-height: 1.4; }

/* 6. Aviso */
.aviso { margin: 0; font-size: 11px; color: var(--clr-text-muted); line-height: 1.5; border-top: 1px solid var(--clr-stroke-soft); padding-top: 10px; }
</style>
