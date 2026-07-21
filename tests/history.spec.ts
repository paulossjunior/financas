import { test, expect } from "@playwright/test";

const INVOICES_MAY = [
  { id: "inv-may-1", filename: "2026-05-fatura-btg.xlsx", month: "2026-05", row_count: 20, imported_at: "2026-05-10T10:00:00" },
  { id: "inv-may-2", filename: "2026-05-fatura-dep.xlsx", month: "2026-05", row_count: 5, imported_at: "2026-05-11T10:00:00" },
];
const INVOICES_MAR = [
  { id: "inv-mar-1", filename: "2026-03-fatura-btg.xlsx", month: "2026-03", row_count: 18, imported_at: "2026-03-10T10:00:00" },
];

const DASHBOARD_ALL = {
  period: { from: "2026-03", to: "2026-05" },
  total_charged: "5000.00", total_reversals: "100.00", net_total: "4900.00",
  invoice_count: 3, categories: [], top_transactions: [],
  monthly_trend: [
    { month: "2026-05", net_total: "3200.00", categories: [] },
    { month: "2026-03", net_total: "1700.00", categories: [] },
  ],
};

const DASHBOARD_MAY = {
  period: { from: "2026-05", to: "2026-05" },
  total_charged: "3300.00", total_reversals: "100.00", net_total: "3200.00",
  invoice_count: 2, categories: [], top_transactions: [],
  monthly_trend: [{ month: "2026-05", net_total: "3200.00", categories: [] }],
};

async function setupMocks(page: any, invoices = [...INVOICES_MAY, ...INVOICES_MAR], dashboard = DASHBOARD_ALL) {
  await page.addInitScript(
    ({ invs, dash }: any) => {
      let currentInvoices = invs;
      let removeCalledWith: string | null = null;
      (window as any).__TAURI_INTERNALS__ = {
        invoke: (cmd: string, args: any) => {
          if (cmd === "list_invoices") return Promise.resolve(currentInvoices);
          if (cmd === "get_dashboard_cmd") {
            const ids: string[] | undefined = args?.filter?.invoice_ids;
            if (ids && ids.length > 0) {
              // Return filtered dashboard
              const filtered = currentInvoices.filter((i: any) => ids.includes(i.id));
              const months = [...new Set(filtered.map((i: any) => i.month))];
              const trend = dash.monthly_trend.filter((s: any) => months.includes(s.month));
              const total = trend.reduce((sum: number, s: any) => sum + parseFloat(s.net_total), 0).toFixed(2);
              return Promise.resolve({ ...dash, net_total: total, monthly_trend: trend, invoice_count: filtered.length });
            }
            return Promise.resolve(dash);
          }
          if (cmd === "get_config") return Promise.resolve({ faturas_directory: "faturas", category_rules: [] });
          if (cmd === "plugin:dialog|message") return Promise.resolve("Yes");
          if (cmd === "remove_invoice") {
            removeCalledWith = args?.invoice_id;
            currentInvoices = currentInvoices.filter((i: any) => i.id !== args?.invoice_id);
            return Promise.resolve(null);
          }
          return Promise.resolve(null);
        },
      };
      (window as any).__TAURI__ = {};
      (window as any).__removedInvoiceId = () => removeCalledWith;
    },
    { invs: invoices, dash: dashboard }
  );
}

test.describe("Histórico — US1: Listagem mensal", () => {
  test("shows month groups sorted descending", async ({ page }) => {
    await setupMocks(page);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    const groups = page.locator(".month-group");
    await expect(groups).toHaveCount(2);

    await expect(groups.nth(0).locator(".month-label")).toContainText("Maio 2026");
    await expect(groups.nth(1).locator(".month-label")).toContainText("Março 2026");
  });

  test("shows invoice count and total for each group", async ({ page }) => {
    await setupMocks(page);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    const mayGroup = page.locator(".month-group").nth(0);
    await expect(mayGroup.locator(".count-badge")).toContainText("2");
    await expect(mayGroup.locator(".group-total")).toContainText("3.200");
  });

  test("shows invoice rows inside each group", async ({ page }) => {
    await setupMocks(page);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    const mayGroup = page.locator(".month-group").nth(0);
    const rows = mayGroup.locator(".invoice-row");
    await expect(rows).toHaveCount(2);
    await expect(rows.nth(0)).toContainText("2026-05-fatura");
  });

  test("shows empty state when no invoices", async ({ page }) => {
    await setupMocks(page, []);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    await expect(page.locator(".empty-state")).toBeVisible();
    await expect(page.locator(".empty-state")).toContainText("Nenhuma fatura importada");
  });
});

test.describe("Histórico — US2: Filtrar dashboard por mês", () => {
  test("clicking 'Ver dashboard' navigates to / with filter badge", async ({ page }) => {
    await setupMocks(page);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    await page.locator("[data-testid='filter-btn']").first().click();

    await page.waitForURL("http://localhost:1420/");
    await expect(page.locator(".filter-badge strong")).toHaveText("Maio/2026");
  });

  test("clear filter button removes badge and reloads unfiltered", async ({ page }) => {
    await setupMocks(page);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    await page.locator("[data-testid='filter-btn']").first().click();
    await page.waitForURL("http://localhost:1420/");

    await page.locator(".clear-filter").click();

    await expect(page.locator(".filter-badge")).not.toBeVisible();
  });
});

test.describe("Histórico — US3: Remover fatura", () => {
  test("remove button triggers confirmation and removes row", async ({ page }) => {
    await setupMocks(page);
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    const mayGroup = page.locator(".month-group").nth(0);
    const initialRows = await mayGroup.locator("[data-testid='remove-btn']").count();
    expect(initialRows).toBe(2);

    await mayGroup.locator("[data-testid='remove-btn']").first().click();

    // After remove, group should have 1 invoice row
    await expect(mayGroup.locator("[data-testid='remove-btn']")).toHaveCount(1);
  });

  test("removing last invoice in group removes entire group", async ({ page }) => {
    await setupMocks(page, [...INVOICES_MAR]); // only march (1 invoice)
    await page.goto("http://localhost:1420/historico");
    await page.waitForTimeout(1500);

    await expect(page.locator(".month-group")).toHaveCount(1);

    await page.locator("[data-testid='remove-btn']").click();

    await expect(page.locator(".empty-state")).toBeVisible({ timeout: 8000 });
  });
});
