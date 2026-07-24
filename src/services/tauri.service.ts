// Single Tauri IPC boundary — typed wrappers around every backend command (the only file that calls invoke).
import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  DashboardData,
  DashboardFilter,
  EntryKind,
  BankEntry,
  ClassifiedEntry,
  ImportResult,
  InflationData,
  InvoiceInfo,
  StatementPreview,
  ManualEntry,
  Payslip,
  Transaction,
  YearSummary,
  RecurringCategoryInfo,
  RecurringSuggestion,
  DerivedFixed,
  PersonalInflationDetail,
  BackupResult,
  RestoreResult,
} from "@/types/api.types";

const ERROR_MESSAGES: Record<string, string> = {
  ENCRYPTED_FILE:
    "Arquivo protegido por senha. Abra no Excel/Numbers, remova a proteção e salve novamente.",
  FILE_NOT_FOUND: "Arquivo não encontrado. Verifique se o caminho está correto.",
  NO_DATA: "Nenhuma fatura importada. Importe um arquivo para continuar.",
  INVOICE_NOT_FOUND: "Fatura não encontrada.",
  DUPLICATE_INVOICE: "Fatura já importada — substituída com os dados mais recentes.",
  INVALID_BACKUP:
    "Arquivo inválido: não é um backup da base do Financas. A base atual não foi alterada.",
  BACKUP_DIR_INVALID: "Pasta de destino inválida. Escolha uma pasta existente.",
  BACKUP_FAILED: "Não foi possível gravar o backup. Verifique permissões e espaço em disco.",
  RESTORE_FAILED: "Não foi possível concluir a restauração. A base anterior foi preservada.",
};

function mapError(raw: string): string {
  for (const [code, msg] of Object.entries(ERROR_MESSAGES)) {
    if (raw.includes(code)) return msg;
  }
  if (raw.startsWith("INVALID_FORMAT:")) {
    const cols = raw.replace("INVALID_FORMAT:", "");
    return `Formato inválido: colunas ausentes — ${cols}`;
  }
  // Keep the technical detail in the console for debugging, but show the user a calm,
  // actionable message instead of a raw error string.
  console.error("[financas] erro inesperado:", raw);
  return "Não foi possível concluir a operação. Tente novamente; se continuar, reinicie o app.";
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

/** Bank statement (.xls) — preview what will be imported vs excluded (no save). */
export async function previewBankStatement(path: string): Promise<StatementPreview> {
  try {
    return await invoke<StatementPreview>("preview_bank_statement", { path });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Import the statement's included entries (dedup). Returns how many were saved. */
export async function importBankStatement(path: string): Promise<number> {
  try {
    return await invoke<number>("import_bank_statement", { path });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Save the (edited) included entries from a preview. Returns how many were saved. */
export async function saveBankStatement(account: string, entries: ClassifiedEntry[]): Promise<number> {
  try {
    return await invoke<number>("save_bank_statement", { account, entries });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Change the category of an already-imported bank entry. */
export async function setBankEntryCategory(id: string, category: string): Promise<void> {
  try {
    await invoke<void>("set_bank_entry_category", { id, category });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function listBankEntries(): Promise<BankEntry[]> {
  try {
    return await invoke<BankEntry[]>("list_bank_entries");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function removeBankEntry(id: string): Promise<void> {
  try {
    await invoke<void>("remove_bank_entry", { id });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function clearBankEntries(): Promise<void> {
  try {
    await invoke<void>("clear_bank_entries");
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

// ── Recurring categories (feature 010) ──

export async function listRecurringCategories(): Promise<RecurringCategoryInfo[]> {
  try {
    return await invoke<RecurringCategoryInfo[]>("list_recurring_categories");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function setCategoryRecurring(
  category: string,
  recurring: boolean,
  startMonth?: string | null,
  endMonth?: string | null,
): Promise<void> {
  try {
    await invoke<void>("set_category_recurring", {
      category,
      recurring,
      startMonth: startMonth ?? null,
      endMonth: endMonth ?? null,
    });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function recurringSuggestions(): Promise<RecurringSuggestion[]> {
  try {
    return await invoke<RecurringSuggestion[]>("recurring_suggestions");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function dismissRecurringSuggestion(target: string): Promise<void> {
  try {
    await invoke<void>("dismiss_recurring_suggestion", { target });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

export async function listFixedExpenses(month: string): Promise<DerivedFixed[]> {
  try {
    return await invoke<DerivedFixed[]>("list_fixed_expenses", { month });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Set (or clear with null) the user's editable base value for a recurring category. */
export async function setRecurringBase(category: string, baseAmount: string | null): Promise<void> {
  try {
    await invoke<void>("set_recurring_base", { category, baseAmount });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** All distinct category names in use (config + card + bank + manual + payslip). */
export async function listAllCategories(): Promise<string[]> {
  try {
    return await invoke<string[]>("list_all_categories");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Detailed personal-inflation breakdown (contributions, comparison, basket/income
 *  impact, behavioral simulation). Null when no indices are cached or no spending. */
export async function getPersonalInflationDetail(): Promise<PersonalInflationDetail | null> {
  try {
    return await invoke<PersonalInflationDetail | null>("get_personal_inflation_detail");
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

// ── Backup & restore (feature 012) ──

/** Back up the whole database into `destDir`; returns the file path written. */
export async function backupDatabase(destDir: string): Promise<BackupResult> {
  try {
    return await invoke<BackupResult>("backup_database", { destDir });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}

/** Replace the current database with the one at `sourcePath` (validated first; the
 *  previous data is saved to a safety copy). Caller should reload the app on success. */
export async function restoreDatabase(sourcePath: string): Promise<RestoreResult> {
  try {
    return await invoke<RestoreResult>("restore_database", { sourcePath });
  } catch (e) {
    throw new Error(mapError(String(e)));
  }
}
