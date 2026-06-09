import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig,
  DashboardData,
  DashboardFilter,
  ImportResult,
  InvoiceInfo,
  Transaction,
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

export async function importInvoices(paths: string[]): Promise<ImportResult[]> {
  try {
    return await invoke<ImportResult[]>("import_invoices", { paths });
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
