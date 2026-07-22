/** Currency mask for text inputs: keep only digits, read them as cents, render
 *  a pt-BR money string ("1.234,56"). Makes a field accept numbers only and always
 *  show a money value. Empty input → "". */
export function maskMoney(raw: string): string {
  const digits = raw.replace(/\D/g, "");
  if (!digits) return "";
  const cents = parseInt(digits, 10);
  if (!Number.isFinite(cents)) return "";
  return (cents / 100).toLocaleString("pt-BR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
}

/** Parse a pt-BR money string ("1.234,56" / "1234,56") to a number. NaN if invalid. */
export function parseMoneyBR(s: string): number {
  const cleaned = s.replace(/[^\d,-]/g, "").replace(",", ".");
  return parseFloat(cleaned);
}
