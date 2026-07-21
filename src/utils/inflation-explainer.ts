// Plain-language inflation explainer. Pure functions: turn observed inflation +
// the user's monthly expense/income into concrete reais + a one-line phrase.
// Projections are estimates (compound growth on observed inflation), not forecasts.

export interface ExplainerInput {
  inflAnnual: number; // IPCA 12 months (%)
  personalMonth: number; // personal monthly inflation (%)
  personalDiff: number; // personal − IPCA (p.p., month)
  monthlyExpense: number; // R$
  monthlyIncome: number; // R$
  topGroup?: string; // biggest-weight category/group
}

export interface Projection {
  label: string;
  cost: string; // formatted future cost
  extra: string; // formatted delta
}

export interface ExplainerItem {
  id: string;
  title: string;
  big: string;
  phrase: string;
  tone: "neutral" | "bad" | "good";
  proj?: Projection[];
}

export interface Explainer {
  available: boolean;
  personalAnnual: number;
  ipcaAnnual: number;
  items: ExplainerItem[];
}

const brl = (v: number) => "R$ " + Math.round(v).toLocaleString("pt-BR");
const pct = (v: number) =>
  v.toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + "%";

/** ((1 + m/100)^12 − 1) × 100 — monthly rate compounded to a yearly rate. */
export function annualizeMonthly(monthlyPct: number): number {
  return (Math.pow(1 + monthlyPct / 100, 12) - 1) * 100;
}

/** value grown by `annualPct` for `years` (compound). */
export function project(value: number, annualPct: number, years: number): number {
  return value * Math.pow(1 + annualPct / 100, years);
}

/** purchasing power of `nominal` after `years` of `annualPct` inflation. */
export function realValue(nominal: number, annualPct: number, years: number): number {
  return nominal / Math.pow(1 + annualPct / 100, years);
}

/** Build the plain-language explainer items from observed inflation + user totals. */
export function buildExplainer(input: ExplainerInput): Explainer {
  const ipcaAnnual = input.inflAnnual;
  const personalAnnual = annualizeMonthly(input.personalMonth);
  const items: ExplainerItem[] = [];

  // 1) Future cost of current expenses (uses the user's own inflation).
  if (input.monthlyExpense > 0) {
    const rate = personalAnnual;
    const proj: Projection[] = [1, 3, 5].map((y) => {
      const c = project(input.monthlyExpense, rate, y);
      return { label: y === 1 ? "12 meses" : `${y} anos`, cost: brl(c), extra: "+" + brl(c - input.monthlyExpense) };
    });
    const c12 = project(input.monthlyExpense, rate, 1);
    items.push({
      id: "expense",
      title: "Seus gastos no futuro",
      big: brl(c12),
      tone: "bad",
      phrase: `Se sua inflação continuar em ~${pct(rate)} ao ano, o que hoje custa ${brl(input.monthlyExpense)} por mês vai custar ~${brl(c12)} em 12 meses (${"+" + brl(c12 - input.monthlyExpense)}). Estimativa "se continuar assim".`,
      proj,
    });
  }

  // 2) Purchasing power of income (uses IPCA).
  if (input.monthlyIncome > 0) {
    const loss = (input.monthlyIncome * ipcaAnnual) / 100;
    const deflation = ipcaAnnual < 0;
    items.push({
      id: "income",
      title: "Poder de compra da renda",
      big: (deflation ? "+" : "−") + brl(Math.abs(loss)),
      tone: deflation ? "good" : "bad",
      phrase: deflation
        ? `Com deflação, sua renda de ${brl(input.monthlyIncome)} ganha ~${brl(Math.abs(loss))} de poder de compra em 12 meses.`
        : `Sem reajuste, sua renda de ${brl(input.monthlyIncome)} perde ~${brl(loss)} de poder de compra em 12 meses. Para empatar com a inflação, precisaria subir ~${pct(ipcaAnnual)}.`,
    });
  }

  // 3) Personal vs official (month).
  {
    const d = input.personalDiff;
    const tone: ExplainerItem["tone"] = d > 0 ? "bad" : d < 0 ? "good" : "neutral";
    const rel = d > 0 ? "a mais" : d < 0 ? "a menos" : "igual";
    const grp = input.topGroup ? ` — puxada por ${input.topGroup}.` : ".";
    items.push({
      id: "vs",
      title: "Você vs. o IPCA",
      big: (d > 0 ? "+" : d < 0 ? "−" : "") + pct(Math.abs(d)).replace("%", " p.p."),
      tone,
      phrase:
        d === 0
          ? "Sua cesta subiu no mesmo ritmo do IPCA neste mês."
          : `Sua cesta subiu ${pct(Math.abs(d)).replace("%", " p.p.")} ${rel} que o IPCA neste mês${grp}`,
    });
  }

  // 4) Value of money.
  {
    const r1 = realValue(100, ipcaAnnual, 1);
    const r5 = realValue(100, ipcaAnnual, 5);
    items.push({
      id: "money",
      title: "Quanto vale R$ 100",
      big: brl(r1),
      tone: "neutral",
      phrase: `Em poder de compra, R$ 100 de hoje valerão ~${brl(r1)} em 12 meses e ~${brl(r5)} em 5 anos, mantida a inflação de ~${pct(ipcaAnnual)} ao ano.`,
    });
  }

  return { available: true, personalAnnual, ipcaAnnual, items };
}
