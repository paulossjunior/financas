import { ref, computed } from "vue";
import { defineStore } from "pinia";
import { getConfig, saveConfig, recategorizeInvoices } from "@/services/tauri.service";
import type { AppConfig, CategoryGroup } from "@/types/api.types";

const DEFAULT_GROUPS: CategoryGroup[] = [
  { name: "Alimentação", keywords: ["IFOOD", "UBER EATS", "RAPPI", "MCDONALDS", "RESTAURANTE", "LANCHONETE", "PADARIA", "PIZZARIA", "DELIVERY", "BURGER", "SUSHI"], priority: 10 },
  { name: "Transporte", keywords: ["UBER", "99", "CABIFY", "POSTO", "COMBUSTIVEL", "GASOLINA", "PEDAGIO", "ESTACIONAMENTO", "METRÔ", "METRO", "ONIBUS", "BUS"], priority: 20 },
  { name: "Saúde", keywords: ["FARMACIA", "DROGARIA", "CLINICA", "HOSPITAL", "LABORATORIO", "MEDICO", "DENTISTA", "UNIMED", "PLANO DE SAUDE", "SULAMERICA", "AMIL"], priority: 30 },
  { name: "Lazer & Entretenimento", keywords: ["NETFLIX", "SPOTIFY", "STEAM", "CINEMA", "INGRESSO", "DISNEY", "HBO", "YOUTUBE", "PRIME", "APPLE TV", "GLOBOPLAY"], priority: 40 },
  { name: "Compras Online", keywords: ["AMAZON", "SHOPEE", "MERCADOLIVRE", "AMERICANAS", "SUBMARINO", "CASAS BAHIA", "MAGALU", "MAGAZINE", "ALIEXPRESS"], priority: 50 },
  { name: "Educação", keywords: ["ESCOLA", "FACULDADE", "CURSO", "LIVRARIA", "UDEMY", "ALURA", "COURSERA", "DUOLINGO"], priority: 60 },
  { name: "Viagem", keywords: ["HOTEL", "AIRBNB", "LATAM", "GOL", "AZUL", "DECOLAR", "BOOKING", "HOSTEL"], priority: 70 },
  { name: "Moradia & Serviços", keywords: ["INTERNET", "TELEFONE", "CLARO", "VIVO", "TIM", "OI ", "ENERGIA", "AGUA", "GAS", "CONDOMINIO", "ALUGUEL"], priority: 80 },
];

function rulesToGroups(config: AppConfig): CategoryGroup[] {
  if (config.category_rules.length === 0) {
    return DEFAULT_GROUPS.map((g) => ({ ...g, keywords: [...g.keywords] }));
  }
  const map = new Map<string, CategoryGroup>();
  for (const rule of config.category_rules) {
    if (!map.has(rule.category)) {
      map.set(rule.category, { name: rule.category, keywords: [...rule.keywords], priority: rule.priority });
    } else {
      const g = map.get(rule.category)!;
      g.keywords = [...g.keywords, ...rule.keywords];
    }
  }
  return Array.from(map.values());
}

function groupsToRules(groups: CategoryGroup[]) {
  return groups.map((g) => ({
    keywords: [...g.keywords],
    category: g.name,
    priority: g.priority,
  }));
}

export const useSettingsStore = defineStore("settings", () => {
  const config = ref<AppConfig>({
    faturas_directory: "faturas",
    category_rules: [],
    transaction_overrides: {},
    manual_entries: [],
  });
  const groups = ref<CategoryGroup[]>([]);
  const saving = ref(false);
  const error = ref<string | null>(null);

  // Merchants/keywords recently removed from a category (UPPERCASE) — highlighted
  // at the top of the "Fila do Outros" (Mapeamento) so they can be re-categorized.
  const recentlyUnmapped = ref<string[]>([]);
  function markUnmapped(keyword: string): void {
    const k = keyword.trim().toUpperCase();
    if (k && !recentlyUnmapped.value.includes(k)) recentlyUnmapped.value = [...recentlyUnmapped.value, k];
  }
  function clearUnmapped(keyword: string): void {
    const k = keyword.trim().toUpperCase();
    recentlyUnmapped.value = recentlyUnmapped.value.filter((x) => x !== k);
  }

  const categoryGroups = computed(() =>
    [...groups.value].sort((a, b) => a.name.localeCompare(b.name, "pt-BR"))
  );

  async function loadConfig(): Promise<void> {
    try {
      config.value = await getConfig();
      groups.value = rulesToGroups(config.value);
    } catch {
      groups.value = DEFAULT_GROUPS.map((g) => ({ ...g, keywords: [...g.keywords] }));
    }
  }

  function addCategory(name: string): void {
    const maxPriority = groups.value.reduce((m, g) => Math.max(m, g.priority), 0);
    groups.value = [...groups.value, { name, keywords: [], priority: maxPriority + 10 }];
  }

  function deleteCategory(name: string): void {
    groups.value = groups.value.filter((g) => g.name !== name);
  }

  function renameCategory(oldName: string, newName: string): void {
    groups.value = groups.value.map((g) =>
      g.name === oldName ? { ...g, name: newName } : g
    );
  }

  function addKeyword(categoryName: string, keyword: string): void {
    groups.value = groups.value.map((g) =>
      g.name === categoryName
        ? { ...g, keywords: [...g.keywords, keyword.trim()] }
        : g
    );
  }

  function removeKeyword(categoryName: string, keyword: string): void {
    groups.value = groups.value.map((g) =>
      g.name === categoryName
        ? { ...g, keywords: g.keywords.filter((k) => k !== keyword) }
        : g
    );
  }

  function getConflict(keyword: string, currentCategoryName: string): string | null {
    const kw = keyword.toUpperCase();
    for (const g of groups.value) {
      if (g.name === currentCategoryName) continue;
      if (g.keywords.some((k) => k.toUpperCase() === kw)) return g.name;
    }
    return null;
  }

  async function saveCategories(): Promise<void> {
    saving.value = true;
    error.value = null;
    try {
      const updatedConfig: AppConfig = {
        ...config.value,
        category_rules: groupsToRules(groups.value),
      };
      await saveConfig(updatedConfig);
      config.value = updatedConfig;
      await recategorizeInvoices();
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
    } finally {
      saving.value = false;
    }
  }

  return {
    config,
    categoryGroups,
    saving,
    error,
    loadConfig,
    addCategory,
    deleteCategory,
    renameCategory,
    addKeyword,
    removeKeyword,
    getConflict,
    saveCategories,
    recentlyUnmapped,
    markUnmapped,
    clearUnmapped,
  };
});
