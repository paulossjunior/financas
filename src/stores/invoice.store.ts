import { defineStore } from "pinia";
import { ref } from "vue";
import type { DashboardData, DashboardFilter, InvoiceInfo, ImportResult } from "@/types/api.types";
import * as tauriService from "@/services/tauri.service";

export const useInvoiceStore = defineStore("invoice", () => {
  const invoices = ref<InvoiceInfo[]>([]);
  const dashboard = ref<DashboardData | null>(null);
  const filter = ref<DashboardFilter>({});
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function importInvoices(paths: string[]): Promise<ImportResult[]> {
    loading.value = true;
    error.value = null;
    try {
      const results = await tauriService.importInvoices(paths);
      await refreshInvoices();
      return results;
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
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
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      throw e;
    } finally {
      loading.value = false;
    }
  }

  async function loadDashboard(newFilter?: DashboardFilter): Promise<void> {
    loading.value = true;
    error.value = null;
    if (newFilter !== undefined) filter.value = newFilter;
    try {
      dashboard.value = await tauriService.getDashboard(filter.value);
    } catch (e) {
      error.value = String(e instanceof Error ? e.message : e);
      dashboard.value = null;
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
    filter,
    loading,
    error,
    importInvoices,
    refreshInvoices,
    removeInvoice,
    loadDashboard,
    clearError,
  };
});
