import { test, expect } from "@playwright/test";
import type { AppConfig } from "../src/types/api.types";

const DEFAULT_CONFIG: AppConfig = {
  faturas_directory: "faturas",
  category_rules: [],
  transaction_overrides: {},
};

const CONFIG_WITH_RULES: AppConfig = {
  faturas_directory: "faturas",
  category_rules: [
    { keywords: ["IFOOD", "RESTAURANTE"], category: "Alimentação", priority: 10 },
    { keywords: ["UBER"], category: "Transporte", priority: 20 },
  ],
  transaction_overrides: {},
};

async function setupMocks(page: any, config: AppConfig = DEFAULT_CONFIG) {
  await page.addInitScript(
    ({ cfg }: { cfg: AppConfig }) => {
      let currentConfig = JSON.parse(JSON.stringify(cfg));
      let savedConfig: AppConfig | null = null;
      let recategorizeCalled = false;
      let overrideCalled: { id: string; category: string } | null = null;
      let removeCalled: string | null = null;

      (window as any).__TAURI_INTERNALS__ = {
        invoke: (cmd: string, args: any) => {
          if (cmd === "get_config") return Promise.resolve(currentConfig);
          if (cmd === "save_config") {
            savedConfig = args?.new_config;
            currentConfig = args?.new_config;
            return Promise.resolve(null);
          }
          if (cmd === "recategorize_invoices") {
            recategorizeCalled = true;
            return Promise.resolve(3);
          }
          if (cmd === "override_transaction_category") {
            overrideCalled = { id: args?.transactionId, category: args?.category };
            return Promise.resolve(null);
          }
          if (cmd === "remove_transaction_override") {
            removeCalled = args?.transactionId;
            return Promise.resolve(null);
          }
          if (cmd === "list_invoices") return Promise.resolve([]);
          if (cmd === "get_dashboard_cmd") return Promise.resolve({
            period: { from: "2026-05", to: "2026-05" },
            total_charged: "0.00", total_reversals: "0.00", net_total: "0.00",
            invoice_count: 0, categories: [], top_transactions: [], monthly_trend: [],
          });
          return Promise.resolve(null);
        },
      };
      (window as any).__TAURI__ = {};
      (window as any).__getSavedConfig = () => savedConfig;
      (window as any).__recategorizeCalled = () => recategorizeCalled;
      (window as any).__overrideCalled = () => overrideCalled;
      (window as any).__removeCalled = () => removeCalled;
    },
    { cfg: config }
  );
}

test.describe("Configurações — US1: Gerenciar Categorias", () => {
  test("shows default categories when category_rules is empty", async ({ page }) => {
    await setupMocks(page, DEFAULT_CONFIG);
    await page.goto("http://localhost:1420/configuracoes");
    await page.waitForTimeout(1000);

    const rows = page.locator("[data-testid='category-row']");
    await expect(rows.first()).toBeVisible();
    const count = await rows.count();
    expect(count).toBeGreaterThan(0);
  });

  test("shows existing category rules from config", async ({ page }) => {
    await setupMocks(page, CONFIG_WITH_RULES);
    await page.goto("http://localhost:1420/configuracoes");
    await page.waitForTimeout(1000);

    const inputs = page.locator("[data-testid='category-name-input']");
    await expect(inputs.first()).toBeVisible();
    const firstValue = await inputs.first().inputValue();
    expect(["Alimentação", "Transporte"]).toContain(firstValue);
  });

  test("add category button adds a new row", async ({ page }) => {
    await setupMocks(page, CONFIG_WITH_RULES);
    await page.goto("http://localhost:1420/configuracoes");
    await page.waitForTimeout(1000);

    const initialCount = await page.locator("[data-testid='category-row']").count();
    await page.locator("[data-testid='add-category-btn']").click();

    await expect(page.locator("[data-testid='category-row']")).toHaveCount(initialCount + 1);
  });

  test("delete button removes the category row", async ({ page }) => {
    await setupMocks(page, CONFIG_WITH_RULES);
    await page.goto("http://localhost:1420/configuracoes");
    await page.waitForTimeout(1000);

    const initialCount = await page.locator("[data-testid='category-row']").count();
    await page.locator("[data-testid='delete-category-btn']").first().click();

    await expect(page.locator("[data-testid='category-row']")).toHaveCount(initialCount - 1);
  });

  test("saving calls save_config then recategorize_invoices", async ({ page }) => {
    await setupMocks(page, CONFIG_WITH_RULES);
    await page.goto("http://localhost:1420/configuracoes");
    await page.waitForTimeout(1000);

    await page.locator("button.save-btn").click();
    await page.waitForTimeout(500);

    const saved = await page.evaluate(() => (window as any).__getSavedConfig());
    expect(saved).not.toBeNull();
    expect(saved.category_rules).toBeDefined();

    const recategorized = await page.evaluate(() => (window as any).__recategorizeCalled());
    expect(recategorized).toBe(true);
  });
});

test.describe("Configurações — US2: Regras de Palavras-Chave", () => {
  test("keyword chips are visible for existing rules", async ({ page }) => {
    await setupMocks(page, CONFIG_WITH_RULES);
    await page.goto("http://localhost:1420/configuracoes");
    await page.waitForTimeout(1000);

    const chips = page.locator(".chip");
    await expect(chips.first()).toBeVisible();
    await expect(chips.first()).toContainText(/IFOOD|RESTAURANTE|UBER/);
  });
});

const FAKE_TX_ID = "aaaaaaaa-0000-0000-0000-000000000001";

const DASHBOARD_WITH_TRANSACTION = {
  period: { from: "2026-05", to: "2026-05" },
  total_charged: "150.00",
  total_reversals: "0.00",
  net_total: "150.00",
  invoice_count: 1,
  categories: [],
  top_transactions: [
    { id: FAKE_TX_ID, date: "2026-05-10", description: "UBER EATS DELIVERY", category: "Alimentação", amount: "150.00" },
  ],
  monthly_trend: [],
};

async function setupDashboardMocks(page: any, config: AppConfig) {
  await page.addInitScript(
    ({ cfg, dashboard }: { cfg: AppConfig; dashboard: any }) => {
      let currentConfig = JSON.parse(JSON.stringify(cfg));
      let overrideCalled: { id: string; category: string } | null = null;
      let removeCalled: string | null = null;

      (window as any).__TAURI_INTERNALS__ = {
        invoke: (cmd: string, args: any) => {
          if (cmd === "get_config") return Promise.resolve(currentConfig);
          if (cmd === "save_config") { currentConfig = args?.new_config; return Promise.resolve(null); }
          if (cmd === "recategorize_invoices") return Promise.resolve(0);
          if (cmd === "override_transaction_category") {
            overrideCalled = { id: args?.transaction_id, category: args?.category };
            return Promise.resolve(null);
          }
          if (cmd === "remove_transaction_override") {
            removeCalled = args?.transaction_id;
            return Promise.resolve(null);
          }
          if (cmd === "list_invoices") return Promise.resolve([{ id: "inv-1", filename: "test.xlsx", month: "2026-05", transaction_count: 1, total_amount: "150.00" }]);
          if (cmd === "get_dashboard_cmd") return Promise.resolve(dashboard);
          return Promise.resolve(null);
        },
      };
      (window as any).__TAURI__ = {};
      (window as any).__overrideCalled = () => overrideCalled;
      (window as any).__removeCalled = () => removeCalled;
    },
    { cfg: config, dashboard: DASHBOARD_WITH_TRANSACTION }
  );
}

test.describe("Dashboard — US3: Sobrescrever Categoria de Transação", () => {
  test("shows override badge when transaction has override in config", async ({ page }) => {
    const configWithOverride: AppConfig = {
      faturas_directory: "faturas",
      category_rules: [],
      transaction_overrides: { [FAKE_TX_ID]: "Transporte" },
    };
    await setupDashboardMocks(page, configWithOverride);
    await page.goto("http://localhost:1420");
    await page.waitForTimeout(1500);

    await expect(page.locator("[data-testid='override-badge']")).toBeVisible();
  });

  test("no override badge when transaction has no override", async ({ page }) => {
    await setupDashboardMocks(page, { faturas_directory: "faturas", category_rules: [], transaction_overrides: {} });
    await page.goto("http://localhost:1420");
    await page.waitForTimeout(1500);

    await expect(page.locator("[data-testid='override-badge']")).not.toBeVisible();
  });

  test("changing category dropdown calls override_transaction_category", async ({ page }) => {
    await setupDashboardMocks(page, { faturas_directory: "faturas", category_rules: [], transaction_overrides: {} });
    await page.goto("http://localhost:1420");
    await page.waitForTimeout(1500);

    const select = page.locator(".cat-select").first();
    await expect(select).toBeVisible();
    const options = await select.locator("option").allInnerTexts();
    const other = options.find((o: string) => o !== "Alimentação");
    if (other) {
      await select.selectOption({ label: other });
      await page.waitForTimeout(500);
      const called = await page.evaluate(() => (window as any).__overrideCalled());
      expect(called).not.toBeNull();
      expect(called.id).toBe(FAKE_TX_ID);
      expect(called.category).toBe(other);
    }
  });

  test("remove override button calls remove_transaction_override", async ({ page }) => {
    const configWithOverride: AppConfig = {
      faturas_directory: "faturas",
      category_rules: [],
      transaction_overrides: { [FAKE_TX_ID]: "Transporte" },
    };
    await setupDashboardMocks(page, configWithOverride);
    await page.goto("http://localhost:1420");
    await page.waitForTimeout(1500);

    await page.locator("[data-testid='remove-override-btn']").click();
    await page.waitForTimeout(500);

    const called = await page.evaluate(() => (window as any).__removeCalled());
    expect(called).toBe(FAKE_TX_ID);
  });
});
