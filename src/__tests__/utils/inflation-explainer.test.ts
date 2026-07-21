import { describe, it, expect } from "vitest";
import { annualizeMonthly, project, realValue, buildExplainer } from "@/utils/inflation-explainer";

describe("inflation-explainer math", () => {
  it("annualizes a monthly rate (compound)", () => {
    // 1% ao mês → ~12.68% ao ano
    expect(annualizeMonthly(1)).toBeCloseTo(12.6825, 3);
    expect(annualizeMonthly(0)).toBe(0);
  });

  it("projects compound growth", () => {
    expect(project(1000, 10, 1)).toBeCloseTo(1100, 6);
    expect(project(1000, 10, 2)).toBeCloseTo(1210, 6);
  });

  it("computes real (purchasing) value", () => {
    expect(realValue(100, 10, 1)).toBeCloseTo(90.909, 2);
  });
});

describe("buildExplainer", () => {
  const base = {
    inflAnnual: 4.64,
    personalMonth: 0.21,
    personalDiff: 0.05,
    monthlyExpense: 10000,
    monthlyIncome: 12000,
    topGroup: "Habitação",
  };

  it("returns all four cards with full data", () => {
    const e = buildExplainer(base);
    expect(e.available).toBe(true);
    const ids = e.items.map((i) => i.id);
    expect(ids).toEqual(["expense", "income", "vs", "money"]);
    // expense projections at 1/3/5 years
    const exp = e.items.find((i) => i.id === "expense")!;
    expect(exp.proj?.length).toBe(3);
  });

  it("hides expense card without spending and income card without income", () => {
    const e = buildExplainer({ ...base, monthlyExpense: 0, monthlyIncome: 0 });
    const ids = e.items.map((i) => i.id);
    expect(ids).not.toContain("expense");
    expect(ids).not.toContain("income");
    expect(ids).toContain("vs");
    expect(ids).toContain("money");
  });

  it("flips income tone on deflation", () => {
    const e = buildExplainer({ ...base, inflAnnual: -1 });
    const income = e.items.find((i) => i.id === "income")!;
    expect(income.tone).toBe("good");
  });

  it("marks personal-above-IPCA as bad and below as good", () => {
    expect(buildExplainer({ ...base, personalDiff: 0.5 }).items.find((i) => i.id === "vs")!.tone).toBe("bad");
    expect(buildExplainer({ ...base, personalDiff: -0.5 }).items.find((i) => i.id === "vs")!.tone).toBe("good");
  });
});
