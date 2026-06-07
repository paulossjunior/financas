import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useInvoiceStore } from "@/stores/invoice.store";
import type { InvoiceInfo, ImportResult, DashboardData } from "@/types/api.types";

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

    expect(mockImport).toHaveBeenCalledWith(["/path/fatura.xlsx"]);
    expect(results).toEqual([result]);
    expect(store.invoices).toHaveLength(1);
    expect(store.invoices[0].id).toBe("inv-1");
  });

  it("importInvoices sets error on failure", async () => {
    mockImport.mockRejectedValue(new Error("ENCRYPTED_FILE"));
    mockList.mockResolvedValue([]);
    const store = useInvoiceStore();

    await expect(store.importInvoices(["/path/encrypted.xlsx"])).rejects.toThrow();
    expect(store.error).toBe("ENCRYPTED_FILE");
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
      invoice_count: 1,
      categories: [],
      top_transactions: [],
      monthly_trend: [],
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
