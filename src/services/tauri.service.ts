import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  DashboardData,
  DashboardFilter,
  EntryKind,
  ImportResult,
  InflationData,
  InvoiceInfo,
  ManualEntry,
  Payslip,
  Transaction,
  YearSummary,
} from "@/types/api.types";

const ERROR_MESSAGES: Record<string, string> = {
  ENCRYPTED_FILE:
    "Arquivo protegido por senha. Abra no Excel/Numbers, remova a proteção e salve novamente.",
  FILE_NOT_FOUND: "Arquivo não encontrado. Verifique se o caminho está correto.",
  NO_DATA: "Nenhuma fatura importada. Importe um arquivo para continuar.",
  INVOICE_NOT_FOUND: "Fatura não encontrada.",
  DUPLICATE_INVOICE: "Fatura já importada — substituída com os dados mais recentes.",
};

function mapError(raw: string): string {
  for (const [code, msg] of Object.entries(ERROR_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.startsWith("INVALID_FORMAT:")) {
    const cols = raw.replace("INVALID_FORMAT:", "");
    return `Formato inválido: colunas ausentes — ${cols}`;
  }
  return `Erro inesperado: ${raw}`;
}

export async function importInvoices(
  paths: string[],
  password?: string,
  remember?: boolean
): Promise<ImportResult[]> {
  try {
    return await invoke<ImportResult[]>("import_invoices", { paths, password, remember });
  } catch (e) {
    const raw = String(e);
    // Preserve these codes so the UI can prompt for a password.
    if (raw.includes("ENCRYPTED_FILE")) throw new Error("ENCRYPTED_FILE");
    if (raw.includes("WRONG_PASSWORD")) throw new Error("WRONG_PASSWORD");
    throw new Error(mapError(raw));
  }
}

/** Whether an invoice password is saved in the OS keychain. */
export async function hasSavedPassword(): Promise<boolean> {
  try {
    return await invoke<boolean>("has_saved_password");
  } catch {
    return false;
  }
}

/** Forget the saved invoice password. */
export async function clearSavedPassword(): Promise<void> {
  try {
    await invoke<void>("clear_saved_password");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function getDashboard(filter?: DashboardFilter): Promise<DashboardData> {
  try {
    return await invoke<DashboardData>("get_dashboard_cmd", { filter });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Whole-period annual view: monthly income vs expense, totals and category ranking.
 *  Pass a start and/or end calendar year to filter; omit both for all data. */
export async function getYearSummary(yearFrom?: number, yearTo?: number): Promise<YearSummary> {
  try {
    return await invoke<YearSummary>("get_year_summary_cmd", {
      yearFrom: yearFrom ?? null,
      yearTo: yearTo ?? null,
    });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Read cached inflation indices (offline) + personal inflation. */
export async function getInflation(): Promise<InflationData> {
  try {
    return await invoke<InflationData>("get_inflation");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** OPT-IN: fetch the latest IPCA from the IBGE, cache locally, return updated data. */
export async function fetchIpca(): Promise<InflationData> {
  try {
    return await invoke<InflationData>("fetch_ipca");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function listInvoices(): Promise<InvoiceInfo[]> {
  try {
    return await invoke<InvoiceInfo[]>("list_invoices");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function removeInvoice(invoiceId: string): Promise<void> {
  try {
    await invoke<void>("remove_invoice", { invoiceId });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  try {
    await invoke<void>("save_config", { newConfig: config });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function recategorizeInvoices(): Promise<number> {
  return invoke<number>("recategorize_invoices_cmd");
}

/** Add a keyword to a category and recategorize all invoices. Returns changed count. */
export async function addCategoryKeyword(keyword: string, category: string): Promise<number> {
  try {
    return await invoke<number>("add_category_keyword", { keyword, category });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function overrideTransactionCategory(
  transactionId: string,
  category: string
): Promise<void> {
  try {
    await invoke<void>("override_transaction_category", { transactionId, category });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function removeTransactionOverride(transactionId: string): Promise<void> {
  try {
    await invoke<void>("remove_transaction_override", { transactionId });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function getAllTransactions(): Promise<Transaction[]> {
  try {
    return await invoke<Transaction[]>("list_all_transactions");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

// ── Manual entries (income & fixed expenses outside the credit card) ──

export interface ManualEntryInput {
  kind: EntryKind;
  description: string;
  amount: string;
  category: string;
  month: string;
  recurring: boolean;
  /** Income only: true = salary, false = bonus. */
  isSalary?: boolean;
}

export async function listManualEntries(): Promise<ManualEntry[]> {
  try {
    return await invoke<ManualEntry[]>("list_manual_entries");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function addManualEntry(input: ManualEntryInput): Promise<ManualEntry> {
  try {
    return await invoke<ManualEntry>("add_manual_entry", { ...input });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function updateManualEntry(
  id: string,
  input: ManualEntryInput
): Promise<ManualEntry> {
  try {
    return await invoke<ManualEntry>("update_manual_entry", { id, ...input });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function removeManualEntry(id: string): Promise<void> {
  try {
    await invoke<void>("remove_manual_entry", { id });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

// ── Contracheque (payslip) ──

/** Parse a payslip PDF (extract + classify) WITHOUT saving — for the confirm modal. */
export async function importPayslip(path: string): Promise<Payslip> {
  try {
    return await invoke<Payslip>("import_payslip", { path });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Persist a (possibly user-corrected) payslip; re-importing a month replaces it. */
export async function savePayslip(payslip: Payslip): Promise<void> {
  try {
    await invoke<void>("save_payslip", { payslip });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function listPayslips(): Promise<Payslip[]> {
  try {
    return await invoke<Payslip[]>("list_payslips");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function removePayslip(month: string): Promise<void> {
  try {
    await invoke<void>("remove_payslip", { month });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}
