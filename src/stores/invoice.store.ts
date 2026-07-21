import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { DashboardData, DashboardFilter, InvoiceInfo, ImportResult, MonthGroup, ManualEntry, Transaction } from "@/types/api.types";
import type { ManualEntryInput } from "@/services/tauri.service";
import * as tauriService from "@/services/tauri.service";

const MONTH_NAMES = ["Janeiro","Fevereiro","Março","Abril","Maio","Junho","Julho","Agosto","Setembro","Outubro","Novembro","Dezembro"];

function formatMonthLabel(month: string): string {
  if (!month || month === "0000-00") return "Mês desconhecido";
  const [year, m] = month.split("-");
  const idx = parseInt(m, 10) - 1;
  return `${MONTH_NAMES[idx] ?? m} ${year}`;
}

export const useInvoiceStore = defineStore("invoice", () => {
  const invoices = ref<InvoiceInfo[]>([]);
  const dashboard = ref<DashboardData | null>(null);
  const manualEntries = ref<ManualEntry[]>([]);
  const allTransactions = ref<Transaction[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const monthFilter = ref<string | null>(null);

  const hasData = computed(
    () => invoices.value.length > 0 || manualEntries.value.length > 0
  );

  const monthGroups = computed<MonthGroup[]>(() => {
    const groups = new Map<string, InvoiceInfo[]>();
    for (const inv of invoices.value) {
      const key = inv.month || "0000-00";
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(inv);
    }

    const trend = dashboard.value?.monthly_trend ?? [];
    const trendMap = new Map(trend.map((s) => [s.month, s.net_total]));

    return [...groups.entries()]
      .map(([month, invs]) => ({
        month,
        label: formatMonthLabel(month),
        invoices: [...invs].sort((a, b) => b.imported_at.localeCompare(a.imported_at)),
        net_total: trendMap.get(month) ?? null,
        invoice_count: invs.length,
      }))
      .sort((a, b) => {
        if (a.month === "0000-00") return 1;
        if (b.month === "0000-00") return -1;
        return b.month.localeCompare(a.month);
      });
  });

  async function importInvoices(paths: string[], password?: string, remember?: boolean): Promise<ImportResult[]> {
    loading.value = true;
    error.value = null;
    try {
      const results = await tauriService.importInvoices(paths, password, remember);
      await refreshInvoices();
      return results;
    } catch (e) {
      const msg = String(e instanceof Error ? e.message : e);
      // Password-flow codes are handled by the UI, not shown in the error bar.
      if (msg !== "ENCRYPTED_FILE" && msg !== "WRONG_PASSWORD") {
        error.value = msg;
      }
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function refreshInvoices(): Promise<void> {
    invoices.value = await tauriService.listInvoices();
  }

  async function removeInvoice(id: string): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      await tauriService.removeInvoice(id);
      await refreshInvoices();
      if (monthFilter.value !== null) {
        const stillExists = invoices.value.some((i) => i.month === monthFilter.value);
        if (!stillExists) monthFilter.value = null;
      }
      await loadDashboard();
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function loadDashboard(explicitFilter?: DashboardFilter): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      let filter: DashboardFilter | undefined;
      if (explicitFilter !== undefined) {
        filter = explicitFilter;
      } else if (monthFilter.value !== null) {
        const ids = invoices.value
          .filter((i) => i.month === monthFilter.value)
          .map((i) => i.id);
        filter = { invoice_ids: ids };
      }
      dashboard.value = await tauriService.getDashboard(filter);
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      dashboard.value = null;
    } finally {
      loading.value = false;
    }
  }

  async function setMonthFilter(month: string | null): Promise<void> {
    monthFilter.value = month;
    await loadDashboard();
  }

  async function loadManualEntries(): Promise<void> {
    manualEntries.value = await tauriService.listManualEntries();
  }

  async function addManualEntry(input: ManualEntryInput): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      await tauriService.addManualEntry(input);
      await loadManualEntries();
      await loadDashboard();
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function updateManualEntry(id: string, input: ManualEntryInput): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      await tauriService.updateManualEntry(id, input);
      await loadManualEntries();
      await loadDashboard();
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function removeManualEntry(id: string): Promise<void> {
    loading.value = true;
    error.value = null;
    try {
      await tauriService.removeManualEntry(id);
      await loadManualEntries();
      await loadDashboard();
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function loadAllTransactions(): Promise<void> {
    allTransactions.value = await tauriService.getAllTransactions();
  }

  /** Map a keyword to a category (adds a rule) and recategorize everything. */
  async function mapKeyword(keyword: string, category: string): Promise<number> {
    loading.value = true;
    error.value = null;
    try {
      const changed = await tauriService.addCategoryKeyword(keyword, category);
      await loadAllTransactions();
      await loadDashboard();
      return changed;
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  function clearError(): void {
    error.value = null;
  }

  return {
    invoices,
    dashboard,
    manualEntries,
    allTransactions,
    hasData,
    loading,
    error,
    monthFilter,
    monthGroups,
    importInvoices,
    refreshInvoices,
    removeInvoice,
    loadDashboard,
    setMonthFilter,
    loadManualEntries,
    addManualEntry,
    updateManualEntry,
    removeManualEntry,
    loadAllTransactions,
    mapKeyword,
    clearError,
  };
});
