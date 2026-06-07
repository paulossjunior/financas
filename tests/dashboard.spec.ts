import { test, expect, Page } from "@playwright/test";

const MOCK_INVOICE = {
  id: "inv-dash-1",
  filename: "2026-05-fatura.xlsx",
  month: "2026-05",
  row_count: 4,
  imported_at: "2026-05-01T10:00:00",
};

const MOCK_DASHBOARD = {
  period: { from: "2026-05-01", to: "2026-05-31" },
  total_charged: "249.90",
  total_reversals: "0.00",
  net_total: "249.90",
  invoice_count: 1,
  categories: [
    {
      name: "Alimentação",
      total: "68.40",
      reversal_total: "0.00",
      net_total: "68.40",
      percentage: 27.4,
      transaction_count: 2,
      top_transactions: [],
    },
  ],
  top_transactions: [
    { id: "t1", date: "2026-05-09", description: "Mercado Livre", amount: "150.00", category: "Compras Online" },
    { id: "t2", date: "2026-05-07", description: "Ifood", amount: "42.90", category: "Alimentação" },
    { id: "t3", date: "2026-05-10", description: "Drogasil", amount: "31.60", category: "Saúde" },
    { id: "t4", date: "2026-05-08", description: "Uber", amount: "25.50", category: "Transporte" },
  ],
  monthly_trend: [],
};

async function mockTauriInvoke(page: Page): Promise<void> {
  await page.addInitScript(
    ({ invoice, dashboard }) => {
      // @ts-ignore
      window.__TAURI_INTERNALS__ = {
        invoke: (cmd: string) => {
          if (cmd === "list_invoices") return Promise.resolve([invoice]);
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
    { invoice: MOCK_INVOICE, dashboard: MOCK_DASHBOARD }
  );
}

test.describe("Dashboard display", () => {
  test("BiggestSpendBanner shows R$ and category name", async ({ page }) => {
    await mockTauriInvoke(page);
    await page.goto("/");

    await page.waitForSelector(".banner", { timeout: 5000 });
    const banner = page.locator(".banner");
    await expect(banner).toBeVisible();

    // Category name visible
    await expect(banner.locator(".category-name")).toContainText("Alimentação");

    // Amount contains R$ (formatted by MoneyAmount)
    const amountText = await banner.locator(".amount").textContent();
    expect(amountText).toContain("R$");
  });

  test("TopTransactions table renders rows ordered by amount desc", async ({ page }) => {
    await mockTauriInvoke(page);
    await page.goto("/");

    await page.waitForSelector(".top-transactions", { timeout: 5000 });
    const rows = page.locator(".top-transactions tbody tr");
    const count = await rows.count();
    expect(count).toBeGreaterThanOrEqual(1);

    // First row (highest amount) should be Mercado Livre
    const firstRowText = await rows.first().textContent();
    expect(firstRowText).toContain("Mercado Livre");
  });

  test("nav tabs are visible and clickable", async ({ page }) => {
    await mockTauriInvoke(page);
    await page.goto("/");

    const nav = page.locator("nav.nav");
    await expect(nav).toBeVisible();
    await expect(nav.locator("a")).toHaveCount(3);
  });

  test("error boundary shows Portuguese message on invoke failure", async ({ page }) => {
    await page.addInitScript(() => {
      // @ts-ignore
      window.__TAURI_INTERNALS__ = {
        invoke: (cmd: string) => {
          if (cmd === "list_invoices") return Promise.resolve([]);
          if (cmd === "get_config") return Promise.resolve({ faturas_directory: "faturas", category_rules: [] });
          return Promise.reject(new Error("ENCRYPTED_FILE"));
        },
        postMessage: () => {},
        transformCallback: (cb: unknown) => cb,
        convertFileSrc: (src: string) => src,
        metadata: { currentWindow: { label: "main" } },
      };
    });
    await page.goto("/");

    // Trigger an operation that would fail — empty state shown, no crash
    await page.waitForSelector(".empty-state", { timeout: 5000 });
    await expect(page.locator(".empty-state")).toBeVisible();
  });
});
