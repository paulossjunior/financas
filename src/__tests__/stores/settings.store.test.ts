import { describe, it, expect, vi, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import type { CategoryGroup } from "@/types/api.types";

vi.mock("@/services/tauri.service", () => ({
  getConfig: vi.fn(),
  saveConfig: vi.fn(),
  recategorizeInvoices: vi.fn(),
}));

import * as tauriService from "@/services/tauri.service";
const mockGetConfig = vi.mocked(tauriService.getConfig);
const mockSaveConfig = vi.mocked(tauriService.saveConfig);
const mockRecategorize = vi.mocked(tauriService.recategorizeInvoices);

beforeEach(() => {
  setActivePinia(createPinia());
  vi.clearAllMocks();
});

async function getStore() {
  const { useSettingsStore } = await import("@/stores/settings.store");
  return useSettingsStore();
}

describe("useSettingsStore", () => {
  describe("categoryGroups computed", () => {
    it("groups category_rules by category name into CategoryGroups", async () => {
      mockGetConfig.mockResolvedValue({
        category_rules: [
          { keywords: ["IFOOD", "RESTAURANTE"], category: "Alimentação", priority: 10 },
          { keywords: ["UBER"], category: "Transporte", priority: 20 },
        ],
        transaction_overrides: {},
        manual_entries: [],
      });

      const store = await getStore();
      await store.loadConfig();

      expect(store.categoryGroups).toHaveLength(2);
      expect(store.categoryGroups[0].name).toBe("Alimentação");
      expect(store.categoryGroups[0].keywords).toEqual(["IFOOD", "RESTAURANTE"]);
      expect(store.categoryGroups[1].name).toBe("Transporte");
    });

    it("pre-fills defaults when category_rules is empty", async () => {
      mockGetConfig.mockResolvedValue({
        category_rules: [],
        transaction_overrides: {},
        manual_entries: [],
      });

      const store = await getStore();
      await store.loadConfig();

      expect(store.categoryGroups.length).toBeGreaterThan(0);
      const names = store.categoryGroups.map((g: CategoryGroup) => g.name);
      expect(names).toContain("Alimentação");
      expect(names).toContain("Transporte");
    });
  });

  describe("addCategory", () => {
    it("appends a new CategoryGroup with empty keywords", async () => {
      mockGetConfig.mockResolvedValue({
        category_rules: [],
        transaction_overrides: {},
        manual_entries: [],
      });

      const store = await getStore();
      await store.loadConfig();
      const initialLen = store.categoryGroups.length;

      store.addCategory("Pets");

      expect(store.categoryGroups).toHaveLength(initialLen + 1);
      const pets = store.categoryGroups.find((g: CategoryGroup) => g.name === "Pets");
      expect(pets).toBeDefined();
      expect(pets?.keywords).toEqual([]);
    });
  });

  describe("deleteCategory", () => {
    it("removes the category group by name", async () => {
      mockGetConfig.mockResolvedValue({
        category_rules: [
          { keywords: ["UBER"], category: "Transporte", priority: 20 },
        ],
        transaction_overrides: {},
        manual_entries: [],
      });

      const store = await getStore();
      await store.loadConfig();

      store.deleteCategory("Transporte");

      const names = store.categoryGroups.map((g: CategoryGroup) => g.name);
      expect(names).not.toContain("Transporte");
    });
  });

  describe("renameCategory", () => {
    it("updates the name of a category group", async () => {
      mockGetConfig.mockResolvedValue({
        category_rules: [
          { keywords: ["IFOOD"], category: "Alimentação", priority: 10 },
        ],
        transaction_overrides: {},
        manual_entries: [],
      });

      const store = await getStore();
      await store.loadConfig();

      store.renameCategory("Alimentação", "Comida");

      const names = store.categoryGroups.map((g: CategoryGroup) => g.name);
      expect(names).toContain("Comida");
      expect(names).not.toContain("Alimentação");
    });
  });

  describe("saveCategories", () => {
    it("calls saveConfig with updated rules then recategorizeInvoices", async () => {
      mockGetConfig.mockResolvedValue({
        category_rules: [],
        transaction_overrides: {},
        manual_entries: [],
      });
      mockSaveConfig.mockResolvedValue(undefined);
      mockRecategorize.mockResolvedValue(3);

      const store = await getStore();
      await store.loadConfig();
      store.addCategory("Pets");

      await store.saveCategories();

      expect(mockSaveConfig).toHaveBeenCalledOnce();
      expect(mockRecategorize).toHaveBeenCalledOnce();

      const savedConfig = mockSaveConfig.mock.calls[0][0];
      const petRule = savedConfig.category_rules.find((r: { category: string }) => r.category === "Pets");
      expect(petRule).toBeDefined();
    });
  });
});
