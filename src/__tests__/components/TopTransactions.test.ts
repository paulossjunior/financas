import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import TopTransactions from "@/components/dashboard/TopTransactions.vue";
import type { TransactionSummary } from "@/types/api.types";

function makeTx(overrides: Partial<TransactionSummary> = {}): TransactionSummary {
  return {
    id: Math.random().toString(),
    date: "2026-05-15",
    description: "Ifood",
    amount: "42.90",
    category: "Alimentação",
    ...overrides,
  };
}

describe("TopTransactions", () => {
  it("renders a row per transaction", () => {
    const txs = [makeTx(), makeTx({ description: "Uber" }), makeTx({ description: "Drogasil" })];
    const wrapper = mount(TopTransactions, { props: { transactions: txs } });
    const rows = wrapper.findAll("tbody tr");
    expect(rows).toHaveLength(3);
  });

  it("renders empty table when no transactions", () => {
    const wrapper = mount(TopTransactions, { props: { transactions: [] } });
    expect(wrapper.findAll("tbody tr")).toHaveLength(0);
  });

  it("shows description in table row", () => {
    const wrapper = mount(TopTransactions, {
      props: { transactions: [makeTx({ description: "Mercado Livre" })] },
    });
    expect(wrapper.find("tbody tr td.desc").text()).toBe("Mercado Livre");
  });

  it("formats date from ISO to dd/mm/yyyy", () => {
    const wrapper = mount(TopTransactions, {
      props: { transactions: [makeTx({ date: "2026-05-20" })] },
    });
    const firstCell = wrapper.find("tbody tr td").text();
    expect(firstCell).toBe("20/05/2026");
  });

  it("renders 5 rows when 5 transactions passed (ordered by caller)", () => {
    const txs = [
      makeTx({ amount: "500.00", description: "A" }),
      makeTx({ amount: "400.00", description: "B" }),
      makeTx({ amount: "300.00", description: "C" }),
      makeTx({ amount: "200.00", description: "D" }),
      makeTx({ amount: "100.00", description: "E" }),
    ];
    const wrapper = mount(TopTransactions, { props: { transactions: txs } });
    const rows = wrapper.findAll("tbody tr");
    expect(rows).toHaveLength(5);
    // First row has highest amount (order is caller's responsibility)
    expect(rows[0].text()).toContain("A");
  });
});
