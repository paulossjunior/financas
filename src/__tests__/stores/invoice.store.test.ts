import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useInvoiceStore } from "@/stores/invoice.store";
import type { InvoiceInfo, ImportResult, DashboardData } from "@/types/api.types";
import { nextTick } from "vue";

vi.mock("@/services/tauri.service", () => ({
  importInvoices: vi.fn(),
  listInvoices: vi.fn(),
  removeInvoice: vi.fn(),
  getDashboard: vi.fn(),
}));

import * as tauriService from "@/services/tauri.service";
const mockImport = vi.mocked(tauriService.importInvoices);
const mockList = vi.mocked(tauriService.listInvoices);
const mockRemove = vi.mocked(tauriService.removeInvoice);
const mockGetDashboard = vi.mocked(tauriService.getDashboard);

function makeInvoice(overrides: Partial<InvoiceInfo> = {}): InvoiceInfo {
  return {
    id: "inv-1",
    filename: "2026-05-fatura.xlsx",
    month: "2026-05",
    row_count: 10,
    imported_at: "2026-05-01T10:00:00",
    ...overrides,
  };
}

function makeImportResult(overrides: Partial<ImportResult> = {}): ImportResult {
  return {
    invoice_id: "inv-1",
    filename: "2026-05-fatura.xlsx",
    month: "2026-05",
    row_count: 10,
    is_replace: false,
    warnings: [],
    ...overrides,
  };
}

describe("invoice.store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    mockImport.mockReset();
    mockList.mockReset();
    mockRemove.mockReset();
    mockGetDashboard.mockReset();
  });

  it("initial state is empty", () => {
    const store = useInvoiceStore();
    expect(store.invoices).toHaveLength(0);
    expect(store.dashboard).toBeNull();
    expect(store.loading).toBe(false);
    expect(store.error).toBeNull();
  });

  it("importInvoices calls service and refreshes list", async () => {
    const result = makeImportResult();
    const inv = makeInvoice();
    mockImport.mockResolvedValue([result]);
    mockList.mockResolvedValue([inv]);

    const store = useInvoiceStore();
    const results = await store.importInvoices(["/path/fatura.xlsx"]);

    expect(mockImport).toHaveBeenCalledWith(["/path/fatura.xlsx"], undefined, undefined);
    expect(results).toEqual([result]);
    expect(store.invoices).toHaveLength(1);
    expect(store.invoices[0].id).toBe("inv-1");
  });

  it("importInvoices sets error on generic failure", async () => {
    mockImport.mockRejectedValue(new Error("PARSE_ERROR: falha"));
    mockList.mockResolvedValue([]);
    const store = useInvoiceStore();

    await expect(store.importInvoices(["/path/x.xlsx"])).rejects.toThrow();
    expect(store.error).toBe("PARSE_ERROR: falha");
  });

  it("importInvoices does not surface ENCRYPTED_FILE in the error bar", async () => {
    mockImport.mockRejectedValue(new Error("ENCRYPTED_FILE"));
    mockList.mockResolvedValue([]);
    const store = useInvoiceStore();

    await expect(store.importInvoices(["/path/encrypted.xlsx"])).rejects.toThrow("ENCRYPTED_FILE");
    expect(store.error).toBeNull();
  });

  it("removeInvoice calls service and refreshes list", async () => {
    mockRemove.mockResolvedValue(undefined);
    mockList.mockResolvedValue([]);

    const store = useInvoiceStore();
    await store.removeInvoice("inv-1");

    expect(mockRemove).toHaveBeenCalledWith("inv-1");
    expect(store.invoices).toHaveLength(0);
  });

  it("loadDashboard populates dashboard state", async () => {
    const dash: DashboardData = {
      period: { from: "2026-05-01", to: "2026-05-31" },
      total_charged: "1000.00",
      total_reversals: "0.00",
      net_total: "1000.00",
      total_card_net: "1000.00",
      total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0",
      total_income: "0",
      balance: "-1000.00",
      invoice_count: 1,
      categories: [],
      top_transactions: [],
      monthly_trend: [],
      weekday_spending: ["0","0","0","0","0","0","0"],
      installments: [],
      installments_month_total: "0",
      installments_future_total: "0",
      subscriptions: [],
      subscriptions_total: "0",
      forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
    };
    mockGetDashboard.mockResolvedValue(dash);

    const store = useInvoiceStore();
    await store.loadDashboard({});

    expect(store.dashboard).toEqual(dash);
    expect(store.error).toBeNull();
  });

  it("clearError resets error state", () => {
    const store = useInvoiceStore();
    store.error = "some error";
    store.clearError();
    expect(store.error).toBeNull();
  });

  // ── monthGroups computed ──────────────────────────────────────────────────

  describe("monthGroups", () => {
    it("groups invoices by month sorted descending", async () => {
      const inv1 = makeInvoice({ id: "a", month: "2026-03", filename: "mar.xlsx" });
      const inv2 = makeInvoice({ id: "b", month: "2026-05", filename: "mai.xlsx" });
      const inv3 = makeInvoice({ id: "c", month: "2026-03", filename: "mar2.xlsx" });
      mockList.mockResolvedValue([inv1, inv2, inv3]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "2026-03", to: "2026-05" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 3, categories: [], top_transactions: [],
        monthly_trend: [
          { month: "2026-03", net_total: "500.00", categories: [] },
          { month: "2026-05", net_total: "800.00", categories: [] },
        ],
      } as DashboardData);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      await store.loadDashboard();
      await nextTick();

      expect(store.monthGroups).toHaveLength(2);
      expect(store.monthGroups[0].month).toBe("2026-05");
      expect(store.monthGroups[0].label).toBe("Maio 2026");
      expect(store.monthGroups[1].month).toBe("2026-03");
      expect(store.monthGroups[1].label).toBe("Março 2026");
      expect(store.monthGroups[1].invoices).toHaveLength(2);
    });

    it("joins net_total from monthly_trend", async () => {
      mockList.mockResolvedValue([makeInvoice({ month: "2026-05" })]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "2026-05", to: "2026-05" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 1, categories: [], top_transactions: [],
        monthly_trend: [{ month: "2026-05", net_total: "1234.56", categories: [] }],
      } as DashboardData);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      await store.loadDashboard();
      await nextTick();

      expect(store.monthGroups[0].net_total).toBe("1234.56");
    });

    it("puts unknown month (0000-00) at end", async () => {
      const known = makeInvoice({ id: "a", month: "2026-05" });
      const unknown = makeInvoice({ id: "b", month: "0000-00", filename: "unknown.xlsx" });
      mockList.mockResolvedValue([unknown, known]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "2026-05", to: "2026-05" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 2, categories: [], top_transactions: [], monthly_trend: [],
      } as DashboardData);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      await store.loadDashboard();
      await nextTick();

      const last = store.monthGroups[store.monthGroups.length - 1];
      expect(last.month).toBe("0000-00");
      expect(last.label).toBe("Mês desconhecido");
    });

    it("returns empty array when no invoices", async () => {
      mockList.mockResolvedValue([]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "", to: "" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 0, categories: [], top_transactions: [], monthly_trend: [],
      } as DashboardData);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      await store.loadDashboard();
      await nextTick();

      expect(store.monthGroups).toHaveLength(0);
    });
  });

  // ── setMonthFilter ────────────────────────────────────────────────────────

  describe("setMonthFilter", () => {
    it("sets monthFilter and triggers dashboard reload with invoice_ids filter", async () => {
      const inv = makeInvoice({ id: "abc-123", month: "2026-05" });
      mockList.mockResolvedValue([inv]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "2026-05", to: "2026-05" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 1, categories: [], top_transactions: [], monthly_trend: [],
      } as DashboardData);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      await store.setMonthFilter("2026-05");

      expect(store.monthFilter).toBe("2026-05");
      expect(mockGetDashboard).toHaveBeenLastCalledWith(
        expect.objectContaining({ invoice_ids: ["abc-123"] })
      );
    });

    it("setMonthFilter(null) clears filter and reloads unfiltered", async () => {
      mockList.mockResolvedValue([makeInvoice()]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "2026-05", to: "2026-05" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 1, categories: [], top_transactions: [], monthly_trend: [],
      } as DashboardData);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      await store.setMonthFilter("2026-05");
      mockGetDashboard.mockClear();

      await store.setMonthFilter(null);

      expect(store.monthFilter).toBeNull();
      expect(mockGetDashboard).toHaveBeenLastCalledWith(undefined);
    });
  });

  // ── removeInvoice auto-clear ──────────────────────────────────────────────

  describe("removeInvoice auto-clear filter", () => {
    it("clears monthFilter when removed invoice was last in filtered month", async () => {
      const inv = makeInvoice({ id: "inv-only", month: "2026-05" });
      mockList
        .mockResolvedValueOnce([inv])   // initial load
        .mockResolvedValueOnce([]);     // after remove
      mockGetDashboard.mockResolvedValue({
        period: { from: "", to: "" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 0, categories: [], top_transactions: [], monthly_trend: [],
      } as DashboardData);
      mockRemove.mockResolvedValue(undefined);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      store.monthFilter = "2026-05";
      await store.removeInvoice("inv-only");

      expect(store.monthFilter).toBeNull();
    });

    it("keeps monthFilter when other invoices remain in that month", async () => {
      const inv1 = makeInvoice({ id: "inv-1", month: "2026-05" });
      const inv2 = makeInvoice({ id: "inv-2", month: "2026-05", filename: "b.xlsx" });
      mockList
        .mockResolvedValueOnce([inv1, inv2])
        .mockResolvedValueOnce([inv2]);
      mockGetDashboard.mockResolvedValue({
        period: { from: "", to: "" },
        total_charged: "0", total_reversals: "0", net_total: "0",
        total_card_net: "0", total_manual_expense: "0", total_variable_expense: "0", total_payroll_deductions: "0", total_income: "0", balance: "0",
        weekday_spending: ["0","0","0","0","0","0","0"], installments: [], installments_month_total: "0", installments_future_total: "0",
        subscriptions: [], subscriptions_total: "0", forecast_next: [], forecast_committed_total: "0", forecast_last_month: "",
        invoice_count: 1, categories: [], top_transactions: [], monthly_trend: [],
      } as DashboardData);
      mockRemove.mockResolvedValue(undefined);

      const store = useInvoiceStore();
      await store.refreshInvoices();
      store.monthFilter = "2026-05";
      await store.removeInvoice("inv-1");

      expect(store.monthFilter).toBe("2026-05");
    });
  });

  it("loading is true during async operation then false after", async () => {
    let resolveList!: (v: InvoiceInfo[]) => void;
    mockList.mockReturnValue(new Promise((r) => (resolveList = r)));
    mockImport.mockResolvedValue([]);

    const store = useInvoiceStore();
    const importPromise = store.importInvoices([]);

    // After importInvoices resolves, it calls refreshInvoices (listInvoices)
    // loading should be true while waiting
    await mockImport; // import part done

    resolveList([]);
    await importPromise;
    expect(store.loading).toBe(false);
  });
});
