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

export interface InstallmentInfo {
  current: number;
  total: number;
}

export interface Transaction {
  id: string;
  invoice_id: string;
  date: string;
  description: string;
  amount: string;
  category: string;
  installment: InstallmentInfo | null;
  is_reversal: boolean;
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

export interface YearMonthPoint {
  month: string;
  income: string;
  card: string;
  fixed: string;
  variable: string;
  payroll: string;
  expense: string;
  balance: string;
}

export interface YearSummary {
  months: YearMonthPoint[];
  income_total: string;
  expense_total: string;
  card_total: string;
  fixed_total: string;
  variable_total: string;
  payroll_total: string;
  balance_total: string;
  avg_expense: string;
  biggest_month: string;
  biggest_month_value: string;
  savings_rate: number;
  active_months: number;
  tx_count: number;
  categories: Category[];
  available_years: number[];
  salary_month: string;
  salary_only: string;
  fixed_month: string;
  card_ceiling: string;
  card_ceiling_salary: string;
}

export interface PayslipItem {
  kind: string; // "rendimento" | "desconto"
  class: string; // "salario" | "bonus" | "wash" | "recorrente"
  description: string;
  amount: string;
  net_share: string;
  offsetting: boolean;
}

export interface Payslip {
  id: string;
  month: string;
  gross: string;
  real_gross: string;
  deductions: string;
  net: string;
  salary_liq: string;
  bonus_liq: string;
  ir_base: string;
  fgts: string;
  items: PayslipItem[];
  source_file: string;
  imported_at: string;
}

export interface DashboardPeriod {
  from: string;
  to: string;
}

export interface DashboardData {
  period: DashboardPeriod;
  total_charged: string;
  total_reversals: string;
  /** Grand total of expenses (card net + manual fixed expenses). Categories sum to this. */
  net_total: string;
  /** Card net only. */
  total_card_net: string;
  /** Recurring manual fixed expenses in scope (contas fixas — excludes payroll). */
  total_manual_expense: string;
  /** One-off (avulso) manual expenses in scope. */
  total_variable_expense: string;
  /** Payroll deductions (folha) in scope. */
  total_payroll_deductions: string;
  /** Manual income (crédito) in scope. */
  total_income: string;
  /** total_income − net_total. Positive = sobra, negative = déficit. */
  balance: string;
  invoice_count: number;
  categories: Category[];
  top_transactions: TransactionSummary[];
  monthly_trend: MonthlySnapshot[];
  /** Card spending by weekday, Monday..Sunday. */
  weekday_spending: string[];
  installments: InstallmentSummary[];
  installments_month_total: string;
  installments_future_total: string;
  subscriptions: SubscriptionSummary[];
  subscriptions_total: string;
}

export interface InstallmentSummary {
  description: string;
  current: number;
  total: number;
  amount: string;
  remaining: number;
}

export interface SubscriptionSummary {
  name: string;
  total: string;
  count: number;
}

export type EntryKind = "income" | "expense";

export interface ManualEntry {
  id: string;
  kind: EntryKind;
  description: string;
  amount: string;
  category: string;
  /** ISO "YYYY-MM". */
  month: string;
  recurring: boolean;
  /** Income only: true = salary, false = bonus. */
  is_salary: boolean;
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
  manual_entries: ManualEntry[];
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
