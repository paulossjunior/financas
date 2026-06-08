export interface ParseWarning {
  row: number;
  message: string;
}

export interface ImportResult {
  invoice_id: string;
  filename: string;
  month: string;
  row_count: number;
  is_replace: boolean;
  warnings: ParseWarning[];
}

export interface InvoiceInfo {
  id: string;
  filename: string;
  month: string;
  due_date?: string;
  row_count: number;
  imported_at: string;
}

export interface DashboardFilter {
  invoice_ids?: string[];
  categories?: string[];
  date_from?: string;
  date_to?: string;
}

export interface TransactionSummary {
  id: string;
  date: string;
  description: string;
  amount: string;
  category: string;
}

export interface Category {
  name: string;
  total: string;
  reversal_total: string;
  net_total: string;
  percentage: number;
  transaction_count: number;
  top_transactions: TransactionSummary[];
}

export interface CategorySnapshot {
  name: string;
  net_total: string;
}

export interface MonthlySnapshot {
  month: string;
  net_total: string;
  categories: CategorySnapshot[];
}

export interface DashboardPeriod {
  from: string;
  to: string;
}

export interface DashboardData {
  period: DashboardPeriod;
  total_charged: string;
  total_reversals: string;
  net_total: string;
  invoice_count: number;
  categories: Category[];
  top_transactions: TransactionSummary[];
  monthly_trend: MonthlySnapshot[];
}

export interface CategoryRule {
  keywords: string[];
  category: string;
  priority: number;
}

export interface AppConfig {
  faturas_directory: string;
  category_rules: CategoryRule[];
  transaction_overrides: Record<string, string>;
}

export interface CategoryGroup {
  name: string;
  keywords: string[];
  priority: number;
}

export interface MonthGroup {
  month: string;
  label: string;
  invoices: InvoiceInfo[];
  net_total: string | null;
  invoice_count: number;
}
