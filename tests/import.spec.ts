import { test, expect, Page } from "@playwright/test";

const MOCK_INVOICE: object = {
  id: "inv-test-1",
  filename: "2026-05-fatura.xlsx",
  month: "2026-05",
  row_count: 5,
  imported_at: "2026-05-01T10:00:00",
};

const MOCK_IMPORT_RESULT: object = {
  invoice_id: "inv-test-1",
  filename: "2026-05-fatura.xlsx",
  month: "2026-05",
  row_count: 5,
  is_replace: false,
  warnings: [],
};

const MOCK_DASHBOARD: object = {
  period: { from: "2026-05-01", to: "2026-05-31" },
  total_charged: "249.90",
  total_reversals: "-5.00",
  net_total: "244.90",
  invoice_count: 1,
  categories: [
    {
      name: "Alimentação",
      total: "68.40",
      reversal_total: "0.00",
      net_total: "68.40",
      percentage: 27.9,
      transaction_count: 2,
      top_transactions: [
        { id: "t1", date: "2026-05-07", description: "Ifood", amount: "42.90", category: "Alimentação" },
      ],
    },
    {
      name: "Transporte",
      total: "25.50",
      reversal_total: "0.00",
      net_total: "25.50",
      percentage: 10.4,
      transaction_count: 1,
      top_transactions: [
        { id: "t2", date: "2026-05-08", description: "Uber", amount: "25.50", category: "Transporte" },
      ],
    },
  ],
  top_transactions: [
    { id: "t3", date: "2026-05-09", description: "Mercado Livre (2/3)", amount: "150.00", category: "Compras Online" },
    { id: "t1", date: "2026-05-07", description: "Ifood", amount: "42.90", category: "Alimentação" },
    { id: "t4", date: "2026-05-10", description: "Drogasil", amount: "31.60", category: "Saúde" },
    { id: "t2", date: "2026-05-08", description: "Uber", amount: "25.50", category: "Transporte" },
    { id: "t5", date: "2026-05-07", description: "Desconto Parcela", amount: "-5.00", category: "Outros" },
  ],
  monthly_trend: [],
};

async function mockTauriInvoke(page: Page): Promise<void> {
  await page.addInitScript(
    ({ invoice, importResult, dashboard }) => {
      // @ts-ignore
      window.__TAURI_INTERNALS__ = {
        invoke: (cmd: string) => {
          if (cmd === "list_invoices") return Promise.resolve([invoice]);
          if (cmd === "import_invoices") return Promise.resolve([importResult]);
          if (cmd === "get_dashboard") return Promise.resolve(dashboard);
          if (cmd === "get_config") return Promise.resolve({ faturas_directory: "faturas", category_rules: [] });
          return Promise.resolve(null);
        },
        postMessage: () => {},
        transformCallback: (cb: unknown) => cb,
        convertFileSrc: (src: string) => src,
        metadata: { currentWindow: { label: "main" } },
      };
    },
    { invoice: MOCK_INVOICE, importResult: MOCK_IMPORT_RESULT, dashboard: MOCK_DASHBOARD }
  );
}

test.describe("Import flow", () => {
  test("dashboard shows category chart after import", async ({ page }) => {
    await mockTauriInvoke(page);
    await page.goto("/");

    // App loads with pre-populated mock (list_invoices returns invoice on mount)
    await page.waitForSelector(".charts-row", { timeout: 5000 });

    // Category chart container renders
    const chartContainer = page.locator(".chart-container").first();
    await expect(chartContainer).toBeVisible();

    // At least one category section exists
    const categorySections = page.locator(".chart-container");
    expect(await categorySections.count()).toBeGreaterThanOrEqual(1);
  });

  test("import button exists on dashboard page", async ({ page }) => {
    await mockTauriInvoke(page);
    await page.goto("/");

    const importBtn = page.locator("button.import-btn");
    await expect(importBtn).toBeVisible();
    await expect(importBtn).toContainText("Importar Faturas");
  });

  test("summary bar shows net total after data loads", async ({ page }) => {
    await mockTauriInvoke(page);
    await page.goto("/");

    await page.waitForSelector(".summary-bar", { timeout: 5000 });
    const summaryBar = page.locator(".summary-bar");
    await expect(summaryBar).toBeVisible();
  });
});
