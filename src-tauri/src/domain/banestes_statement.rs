//! Domain: Banestes checking-account statement parsing (pure).
//!
//! [`ExtratoBanestes`] is the typed picture of one statement: metadata, every balance
//! and declared total the PDF prints, and the movements. The pipeline is explicit —
//! `parse` (text → struct), `conferir` (integrity checks → [`Conferencia`]),
//! `into_parsed` (→ the shared [`ParsedStatement`] the BTG reader also produces) — so
//! classification, categorization, dedup and persistence stay shared downstream.
//!
//! Focus is **entradas e saídas**: balance lines, the summary block and the footer are
//! dropped, and the extracted movements are reconciled against the totals the statement
//! declares. A PDF is a fragile input, so a lost line must become an error, never a
//! cheaper month — and a statement whose checks **cannot run** is refused the same way
//! (see [`Conferencia::exigir`]), because an unverifiable import is a silent risk.
//!
//! Statement grammar (see `specs/014-banestes-statement-adapter/research.md` R2):
//!
//! ```text
//! Agência: 12 - CENTRO Conta: 1234567-8
//! Cliente: <titular> Período: 01/07/2026 à 25/07/2026
//! Data Lançamento Valor (R$)
//! Saldo Anterior  7.337,41
//! 03  Pix Enviado 03/07/2026-21:49:38 1000000000001 <contraparte> - 700,00
//! JUL/26 Saldo  6.637,41
//! ```

use std::sync::OnceLock;

use regex::Regex;
use rust_decimal::Decimal;

use super::bank_statement::{norm, parse_amount, ParsedStatement, RawEntry};

fn re_ag_conta() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)ag[êe]ncia:\s*(\S+).*?conta:\s*(\S+)").unwrap())
}
fn re_cliente() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)cliente:\s*(.+?)(?:\s+per[íi]odo:.*)?$").unwrap())
}
/// Money at the end of a line, optional minus (Banestes prints "- 700,00").
fn re_money_end() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(-\s*)?(\d{1,3}(?:\.\d{3})*,\d{2})\s*$").unwrap())
}
fn re_money() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\d{1,3}(?:\.\d{3})*,\d{2}").unwrap())
}
/// Operation stamp inside the movement text: `03/07/2026-21:49:38` (time optional).
fn re_op_date() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\d{2})/(\d{2})/(\d{4})(?:-\d{2}:\d{2}:\d{2})?").unwrap())
}
/// Posting-day column, e.g. `03  Pix Enviado …`.
fn re_day_prefix() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(\d{1,2})\s+").unwrap())
}
/// Month/year column, e.g. `JUL/26 Saldo …`.
fn re_month_prefix() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^([A-Za-z]{3})/(\d{2})\s*").unwrap())
}

/// True when this text looks like a Banestes checking-account statement.
///
/// The word "Banestes" is a logo image and never reaches the extracted text, so
/// detection is structural. A SouGov.br payslip — the other PDF the user keeps in
/// the same folder — matches none of these markers.
pub fn is_banestes_statement(text: &str) -> bool {
    let n = norm(text);
    n.contains("EXTRATO DE CONTA CORRENTE") && (n.contains("SALDO ANTERIOR") || n.contains("AGENCIA:"))
}

/// Typed picture of one Banestes statement: everything the PDF declares, before it
/// becomes the shared [`ParsedStatement`].
///
/// Keeping the balances and declared totals as fields (instead of locals inside a
/// parse function) makes the integrity rules inspectable and testable on their own:
/// `conferir` can be exercised against statements the parser accepts but should not
/// trust, and a future preview UI can show *which* checks ran.
#[derive(Debug, Default)]
pub struct ExtratoBanestes {
    pub agencia: String,
    pub conta: String,
    pub titular: String,
    /// "Saldo Anterior" — first line of the table.
    pub saldo_anterior: Option<Decimal>,
    /// "Saldo Conta" — footer; the checking account alone.
    pub saldo_conta: Option<Decimal>,
    /// "Saldo Total" — footer; on a consolidated statement this sums *every* product
    /// (poupança, investimento…), not just the account the movements belong to.
    pub saldo_total: Option<Decimal>,
    /// Summary block at the top: total credited in the period.
    pub entradas_declaradas: Option<Decimal>,
    /// Summary block at the top: total debited in the period.
    pub saidas_declaradas: Option<Decimal>,
    pub movimentos: Vec<RawEntry>,
}

/// Outcome of one integrity check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checagem {
    Fechou,
    /// The numbers disagree by this much (statement minus what was read).
    Divergiu { diferenca: Decimal },
    /// The statement did not print the data this check needs (`faltou` names it).
    SemDados { faltou: &'static str },
}

/// The two independent checks a Banestes statement allows.
///
/// `saldos` catches lost/duplicated/misread lines (the running balance stops adding
/// up); `entradas_saidas` additionally catches a flipped sign, which keeps the net
/// sum plausible but moves value between the two columns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conferencia {
    pub saldos: Checagem,
    pub entradas_saidas: Checagem,
}

impl Conferencia {
    /// Strict policy: anything other than `Fechou` on both checks is an error.
    ///
    /// `Divergiu` means the parser misread the statement. `SemDados` is refused too —
    /// deliberately: every real Banestes statement prints the balances and the summary
    /// block, so their absence means the text extraction changed shape, and importing
    /// without the safety net would hide exactly the failures the net exists to catch.
    /// The old behaviour (skip the check silently) turned grammar rot into quietly
    /// unverified months.
    pub fn exigir(&self) -> Result<(), String> {
        let checks = [
            (&self.saldos, "com os saldos"),
            (&self.entradas_saidas, "com as entradas e saídas declaradas"),
        ];
        for (checagem, o_que) in checks {
            match checagem {
                Checagem::Fechou => {}
                Checagem::Divergiu { diferenca } => return Err(reconcile_error(o_que, *diferenca)),
                Checagem::SemDados { faltou } => {
                    return Err(format!(
                        "Não encontrei {faltou} neste extrato para conferir a leitura. \
                         Nada foi importado."
                    ))
                }
            }
        }
        Ok(())
    }
}

impl ExtratoBanestes {
    /// Parse the text of a Banestes statement PDF. Structure only — no integrity
    /// judgment here; call [`Self::conferir`] before trusting the movements.
    pub fn parse(text: &str) -> Result<Self, String> {
        if !is_banestes_statement(text) {
            return Err("Este PDF não é um extrato do Banestes.".into());
        }

        let lines: Vec<&str> = text.lines().collect();
        let header = lines
            .iter()
            .position(|l| is_table_header(l))
            .ok_or("Não reconheci o formato deste extrato Banestes.")?;

        let mut this = ExtratoBanestes::default();
        for l in &lines[..=header] {
            if this.conta.is_empty() {
                if let Some(c) = re_ag_conta().captures(l) {
                    this.agencia = c[1].trim().to_string();
                    this.conta = c[2].trim().to_string();
                }
            }
            if this.titular.is_empty() && norm(l).starts_with("CLIENTE:") {
                if let Some(c) = re_cliente().captures(l) {
                    this.titular = c[1].trim().to_string();
                }
            }
        }
        if let Some((entradas, saidas)) = declared_totals(&lines[..header]) {
            this.entradas_declaradas = Some(entradas);
            this.saidas_declaradas = Some(saidas);
        }

        let mut day: Option<u32> = None;
        let mut month_year: Option<(u32, i32)> = None;
        let mut pending: Option<String> = None;

        for raw in &lines[header + 1..] {
            let mut rest = raw.trim();
            if rest.is_empty() {
                continue;
            }
            // Consume the leading Data column (day and/or MMM/YY), which may repeat.
            loop {
                if let Some(c) = re_month_prefix().captures(rest) {
                    if let Some(m) = month_from_abbr(&c[1]) {
                        let yy: i32 = c[2].parse().unwrap_or(0);
                        month_year = Some((m, 2000 + yy));
                        rest = &rest[c[0].len()..];
                        continue;
                    }
                }
                if let Some(c) = re_day_prefix().captures(rest) {
                    // Only a bare day column — never the start of a money-only line.
                    if let Ok(d) = c[1].parse::<u32>() {
                        if (1..=31).contains(&d) {
                            day = Some(d);
                            rest = &rest[c[0].len()..];
                            continue;
                        }
                    }
                }
                break;
            }
            let rest = rest.trim();
            if rest.is_empty() {
                continue;
            }

            let n = norm(rest);
            if n.starts_with("SALDO") {
                // Balance lines carry state, never movements. Kinds we don't track
                // ("Saldo Poupança" on a consolidated statement) are simply dropped.
                let value = re_money().find(rest).and_then(|m| parse_amount(m.as_str()));
                if n.starts_with("SALDO ANTERIOR") && this.saldo_anterior.is_none() {
                    this.saldo_anterior = value;
                } else if n.starts_with("SALDO TOTAL") {
                    this.saldo_total = value;
                } else if n.starts_with("SALDO CONTA") {
                    this.saldo_conta = value;
                }
                pending = None;
                continue;
            }
            if n.contains("EXTRATO CONSOLIDADO") || n.contains("DATA/HORA EMISSAO") {
                continue;
            }

            // Wrapped movement: the description spilled to the next line, taking the
            // value with it. Join and interpret as one movement.
            let joined = match pending.take() {
                Some(p) => format!("{p} {rest}"),
                None => rest.to_string(),
            };
            let Some(m) = re_money_end().captures(&joined) else {
                // No value yet: keep it only if it actually opens a movement.
                if re_op_date().is_match(&joined) {
                    pending = Some(joined);
                }
                continue;
            };

            let negative = m.get(1).is_some();
            let Some(magnitude) = parse_amount(&m[2]) else {
                continue;
            };
            let amount = if negative { -magnitude } else { magnitude };
            let head = joined[..m.get(0).unwrap().start()].trim().to_string();
            let (transaction, description, date) = split_movement(&head, day, month_year);
            let Some(date) = date else { continue };

            this.movimentos.push(RawEntry {
                month: date[..7].to_string(),
                date,
                btg_category: String::new(),
                transaction,
                description,
                amount,
            });
        }

        if this.movimentos.is_empty() {
            return Err("O extrato não tem lançamentos no período.".into());
        }
        Ok(this)
    }

    /// Run both integrity checks. Pure report — the policy of what to do with a
    /// check that failed or could not run lives in [`Conferencia::exigir`].
    pub fn conferir(&self) -> Conferencia {
        let total: Decimal = self.movimentos.iter().map(|e| e.amount).sum();

        // The Banestes PDF is the *consolidated* export: "Saldo Total" sums every
        // product (poupança, investimento…). The balance that closes against the
        // checking-account movements is "Saldo Conta"; "Saldo Total" is only a
        // fallback for a statement that doesn't print the per-account line.
        let saldo_final = self.saldo_conta.or(self.saldo_total);
        let saldos = match (self.saldo_anterior, saldo_final) {
            (Some(prev), Some(last)) => {
                let diferenca = last - (prev + total);
                if diferenca.is_zero() {
                    Checagem::Fechou
                } else {
                    Checagem::Divergiu { diferenca }
                }
            }
            (None, _) => Checagem::SemDados { faltou: "o saldo anterior" },
            (_, None) => Checagem::SemDados { faltou: "o saldo final" },
        };

        let entradas_saidas = match (self.entradas_declaradas, self.saidas_declaradas) {
            (Some(entradas), Some(saidas)) => {
                let cred: Decimal =
                    self.movimentos.iter().map(|e| e.amount).filter(|a| a.is_sign_positive()).sum();
                let deb: Decimal =
                    self.movimentos.iter().map(|e| e.amount).filter(|a| a.is_sign_negative()).sum();
                if cred == entradas && deb.abs() == saidas {
                    Checagem::Fechou
                } else {
                    let diferenca =
                        if cred != entradas { cred - entradas } else { deb.abs() - saidas };
                    Checagem::Divergiu { diferenca }
                }
            }
            _ => Checagem::SemDados { faltou: "o quadro de entradas e saídas" },
        };

        Conferencia { saldos, entradas_saidas }
    }

    /// Converge on the shared statement model the whole downstream pipeline eats.
    pub fn into_parsed(self) -> ParsedStatement {
        ParsedStatement {
            bank: "Banestes".to_string(),
            // Frozen format: bank-entry ids hash `account` (see
            // `bank_statement::entry_key`) — changing this string re-imports every
            // persisted Banestes entry as new rows.
            account: format!("{}/{}", self.agencia, self.conta),
            holder: self.titular,
            entries: self.movimentos,
        }
    }
}

/// Parse + verify the text of a Banestes statement PDF into entradas/saídas.
///
/// Convenience for the import paths: strict — a statement that fails (or cannot run)
/// the integrity checks is refused with a user-facing pt-BR message.
pub fn parse_banestes_text(text: &str) -> Result<ParsedStatement, String> {
    let extrato = ExtratoBanestes::parse(text)?;
    extrato.conferir().exigir()?;
    Ok(extrato.into_parsed())
}

fn reconcile_error(what: &str, diff: Decimal) -> String {
    format!(
        "A leitura do extrato não fechou {what} (diferença de R$ {}). Nada foi importado.",
        format_brl(diff.abs())
    )
}

/// `1234.5` → `1.234,50` — for error messages the user can compare with the PDF.
fn format_brl(v: Decimal) -> String {
    let s = format!("{:.2}", v);
    let (int, dec) = s.split_once('.').unwrap_or((s.as_str(), "00"));
    let mut grouped = String::new();
    for (i, c) in int.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(c);
    }
    format!("{},{}", grouped.chars().rev().collect::<String>(), dec)
}

fn is_table_header(line: &str) -> bool {
    let n = norm(line);
    n.contains("DATA") && n.contains("LANCAMENTO") && n.contains("VALOR")
}

/// The summary block above the table: saldo total, entradas, saídas (in this order).
fn declared_totals(pre_header: &[&str]) -> Option<(Decimal, Decimal)> {
    let start = pre_header.iter().position(|l| norm(l).contains("ENTRADAS E SAIDAS"))?;
    let mut values = Vec::new();
    for l in &pre_header[start..] {
        for m in re_money().find_iter(l) {
            if let Some(v) = parse_amount(m.as_str()) {
                values.push(v);
            }
        }
        if values.len() >= 3 {
            break;
        }
    }
    if values.len() < 3 {
        return None;
    }
    Some((values[1], values[2]))
}

/// Split a movement's text into operation type, counterparty and date.
///
/// The operation stamp is the divider: what precedes it is the operation type, what
/// follows it is the bank's document number (dropped — noise for the user, and it
/// would pollute the categorization queue) plus the counterparty.
fn split_movement(
    head: &str,
    day: Option<u32>,
    month_year: Option<(u32, i32)>,
) -> (String, String, Option<String>) {
    if let Some(c) = re_op_date().captures(head) {
        let m = c.get(0).unwrap();
        let transaction = head[..m.start()].trim().to_string();
        let after = head[m.end()..].trim();
        let description = after
            .split_whitespace()
            .skip_while(|t| t.chars().all(|c| c.is_ascii_digit()))
            .collect::<Vec<_>>()
            .join(" ");
        let date = format!("{}-{}-{}", &c[3], &c[2], &c[1]);
        let description = if description.is_empty() { transaction.clone() } else { description };
        return (transaction, description, Some(date));
    }
    // No operation stamp (tarifas, débito automático): the whole text is the
    // description, and the Data column gives the date.
    let date = match (day, month_year) {
        (Some(d), Some((m, y))) => Some(format!("{y:04}-{m:02}-{d:02}")),
        _ => None,
    };
    (head.to_string(), head.to_string(), date)
}

fn month_from_abbr(abbr: &str) -> Option<u32> {
    match abbr.to_uppercase().as_str() {
        "JAN" => Some(1), "FEV" => Some(2), "MAR" => Some(3), "ABR" => Some(4),
        "MAI" => Some(5), "JUN" => Some(6), "JUL" => Some(7), "AGO" => Some(8),
        "SET" => Some(9), "OUT" => Some(10), "NOV" => Some(11), "DEZ" => Some(12),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::path::PathBuf;
    use std::str::FromStr;

    fn fixture(name: &str) -> String {
        let p: PathBuf = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("tests/fixtures")
            .join(name);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("fixture {p:?}: {e}"))
    }

    fn main_fixture() -> String {
        fixture("banestes_extrato.txt")
    }

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    const PAYSLIP_TEXT: &str = "\
Comprovante de Rendimentos
SouGov.br
JUL 2026
Vencimento Base 12.345,67
Total de Rendimentos 12.345,67
Total de Descontos 2.000,00
Líquido a Receber 10.345,67";

    // T012
    #[test]
    fn detects_banestes_and_rejects_payslip() {
        assert!(is_banestes_statement(&main_fixture()));
        assert!(!is_banestes_statement(PAYSLIP_TEXT));
        assert!(!is_banestes_statement(""));
    }

    // T013
    #[test]
    fn reads_metadata_and_all_movements() {
        let p = parse_banestes_text(&main_fixture()).unwrap();
        assert_eq!(p.bank, "Banestes");
        assert_eq!(p.account, "12/1234567-8");
        assert_eq!(p.holder, "MARIA APARECIDA DA SILVA SOUZA");
        assert_eq!(p.entries.len(), 9, "9 lançamentos no período");
    }

    // T014
    #[test]
    fn sums_saidas_and_drops_balance_lines() {
        let p = parse_banestes_text(&main_fixture()).unwrap();
        let total: Decimal = p.entries.iter().map(|e| e.amount).sum();
        assert_eq!(total, dec("-7106.11"));
        for e in &p.entries {
            let d = e.description.to_uppercase();
            assert!(!d.contains("SALDO"), "linha de saldo virou lançamento: {d}");
            assert!(!d.contains("EXTRATO CONSOLIDADO"), "rodapé virou lançamento: {d}");
        }
    }

    // T015
    #[test]
    fn joins_wrapped_description_into_one_entry() {
        let p = parse_banestes_text(&main_fixture()).unwrap();
        let e = p
            .entries
            .iter()
            .find(|e| e.amount == dec("-2729.78"))
            .expect("lançamento quebrado em duas linhas");
        assert_eq!(e.description, "ALFA COMERCIO E REPRESENTACOES LTDA");
        assert_eq!(
            p.entries.iter().filter(|e| e.description.contains("ALFA")).count(),
            1
        );
        // the other wrapped line, from a public body
        assert!(p
            .entries
            .iter()
            .any(|e| e.description == "DEPARTAMENTO ESTADUAL DE TRANSITO DO ESP"
                && e.amount == dec("-427.86")));
    }

    // T016
    #[test]
    fn operation_date_wins_over_posting_day() {
        let p = parse_banestes_text(&main_fixture()).unwrap();
        assert!(p.entries.iter().all(|e| e.month == "2026-07"));
        let e = p
            .entries
            .iter()
            .find(|e| e.amount == dec("-800"))
            .expect("lançamento do dia 20");
        assert_eq!(e.date, "2026-07-19", "vale a data da operação, não o dia da coluna");
        assert_eq!(p.entries[0].date, "2026-07-03");
    }

    // T016 (fallback)
    #[test]
    fn line_without_operation_date_falls_back_to_column_day() {
        let p = parse_banestes_text(&fixture("banestes_extrato_credito.txt")).unwrap();
        let e = p
            .entries
            .iter()
            .find(|e| e.amount == dec("-21.90"))
            .expect("tarifa sem data de operação");
        assert_eq!(e.date, "2026-06-20");
        assert_eq!(e.month, "2026-06");
    }

    // T017
    #[test]
    fn splits_operation_type_from_counterparty() {
        let p = parse_banestes_text(&main_fixture()).unwrap();
        let e = p
            .entries
            .iter()
            .find(|e| e.amount == dec("-128.47"))
            .expect("último lançamento");
        assert_eq!(e.transaction, "Pix Enviado");
        assert_eq!(e.description, "GIGA MAIS FIBRA");
        assert!(e.btg_category.is_empty(), "Banestes não informa categoria");
        // no time, no document number leaking into what the user reads
        assert!(!e.description.contains(':'));
        assert!(!e.description.chars().any(|c| c.is_ascii_digit()));
    }

    // T018
    #[test]
    fn unsigned_value_is_an_entrada() {
        let p = parse_banestes_text(&fixture("banestes_extrato_credito.txt")).unwrap();
        let salary = p
            .entries
            .iter()
            .find(|e| e.amount == dec("8000"))
            .expect("crédito de salário");
        assert!(salary.amount > Decimal::ZERO);
        assert_eq!(salary.transaction, "Credito Salario");
        let entradas: Decimal = p.entries.iter().filter(|e| e.amount > Decimal::ZERO).map(|e| e.amount).sum();
        assert_eq!(entradas, dec("8500"));
    }

    // T019
    #[test]
    fn refuses_statement_that_does_not_reconcile() {
        let err = parse_banestes_text(&fixture("banestes_extrato_quebrado.txt")).unwrap_err();
        assert!(
            err.contains("não fech") && err.contains("importado"),
            "mensagem deve explicar e avisar que nada foi importado: {err}"
        );
        assert!(err.contains("10,00"), "mostra a diferença: {err}");
    }

    // T019
    #[test]
    fn refuses_non_banestes_and_headerless_text() {
        assert!(parse_banestes_text(PAYSLIP_TEXT)
            .unwrap_err()
            .contains("não é um extrato do Banestes"));

        let no_header = "Extrato de Conta Corrente\nSaldo Anterior 100,00\nAgência: 12 - CENTRO Conta: 1-1\n";
        assert!(parse_banestes_text(no_header).is_err());
    }

    // T019
    #[test]
    fn reports_period_without_movements() {
        let empty = "\
Extrato de Conta Corrente
SALDO TOTAL ENTRADAS E SAÍDAS
R$ 100,00                  R$ 0,00
                      R$ 0,00
Agência: 12 - CENTRO Conta: 1234567-8
Cliente: MARIA APARECIDA DA SILVA SOUZA Período: 01/05/2026 à 31/05/2026
Data Lançamento Valor (R$)
Saldo Anterior  100,00
Saldos
 Saldo Conta  100,00
Saldo Total  100,00
";
        let err = parse_banestes_text(empty).unwrap_err();
        assert!(err.contains("lançamento"), "mensagem informativa: {err}");
    }

    // ---- ExtratoBanestes: saldos tipados e conferência explícita ----

    #[test]
    fn expoe_saldos_e_totais_declarados() {
        let e = ExtratoBanestes::parse(&main_fixture()).unwrap();
        assert_eq!(e.agencia, "12");
        assert_eq!(e.conta, "1234567-8");
        assert_eq!(e.saldo_anterior, Some(dec("7337.41")));
        assert_eq!(e.saldo_conta, Some(dec("231.30")));
        assert_eq!(e.saldo_total, Some(dec("231.30")));
        assert_eq!(e.entradas_declaradas, Some(dec("0.00")));
        assert_eq!(e.saidas_declaradas, Some(dec("7106.11")));
        let c = e.conferir();
        assert_eq!(c.saldos, Checagem::Fechou);
        assert_eq!(c.entradas_saidas, Checagem::Fechou);
    }

    /// Regressão do extrato consolidado: "Saldo Total" soma poupança/investimento;
    /// o que fecha com os movimentos da conta corrente é "Saldo Conta". Antes a
    /// conferência preferia o Total e um extrato válido com poupança era recusado.
    #[test]
    fn extrato_consolidado_confere_pelo_saldo_da_conta() {
        let text = fixture("banestes_extrato_consolidado.txt");
        let e = ExtratoBanestes::parse(&text).unwrap();
        assert_eq!(e.saldo_conta, Some(dec("231.30")));
        assert_eq!(e.saldo_total, Some(dec("5231.30")), "total inclui a poupança");
        assert_eq!(e.conferir().saldos, Checagem::Fechou, "confere pelo saldo da conta");

        let p = parse_banestes_text(&text).unwrap();
        assert_eq!(p.entries.len(), 9, "linha 'Saldo Poupança' não vira lançamento");
    }

    /// Conferência que não pode rodar é recusa, não importação sem verificação: no
    /// dia em que o texto extraído mudar de forma e a regex de saldo parar de casar,
    /// o usuário vê um erro — em vez de o extrato passar sem rede em silêncio.
    #[test]
    fn sem_saldo_anterior_recusa_em_vez_de_importar_sem_conferir() {
        let text = main_fixture().replace("Saldo Anterior  7.337,41", "");
        let e = ExtratoBanestes::parse(&text).unwrap();
        assert_eq!(e.conferir().saldos, Checagem::SemDados { faltou: "o saldo anterior" });

        let err = parse_banestes_text(&text).unwrap_err();
        assert!(
            err.contains("saldo anterior") && err.contains("Nada foi importado"),
            "{err}"
        );
    }

    #[test]
    fn sem_quadro_de_totais_recusa() {
        let text = main_fixture().replace("SALDO TOTAL ENTRADAS E SAÍDAS", "");
        let e = ExtratoBanestes::parse(&text).unwrap();
        assert_eq!(
            e.conferir().entradas_saidas,
            Checagem::SemDados { faltou: "o quadro de entradas e saídas" }
        );
        assert!(parse_banestes_text(&text).unwrap_err().contains("entradas e saídas"));
    }

    // ---- US2: the Banestes path must go through the BTG rules, unchanged ----

    fn classified(fixture_name: &str, payslip_months: &[&str]) -> Vec<super::super::ClassifiedEntry> {
        use super::super::{classify_statement, Categorizer};
        let p = parse_banestes_text(&fixture(fixture_name)).unwrap();
        let months: std::collections::HashSet<String> =
            payslip_months.iter().map(|s| s.to_string()).collect();
        classify_statement(&p, &Categorizer::with_defaults(), &months)
    }

    fn find_by_amount<'a>(
        cl: &'a [super::super::ClassifiedEntry],
        amount: &str,
    ) -> &'a super::super::ClassifiedEntry {
        cl.iter()
            .find(|c| c.amount == dec(amount))
            .unwrap_or_else(|| panic!("lançamento {amount} não encontrado"))
    }

    // T030
    #[test]
    fn card_bill_payment_is_excluded() {
        let cl = classified("banestes_extrato_credito.txt", &[]);
        let c = find_by_amount(&cl, "-1200");
        assert!(!c.included && c.reason == "fatura", "{c:?}");
    }

    // T031
    #[test]
    fn salary_is_excluded_only_when_a_payslip_exists() {
        let with = classified("banestes_extrato_credito.txt", &["2026-06"]);
        let c = find_by_amount(&with, "8000");
        assert!(!c.included && c.reason == "salario");

        let without = classified("banestes_extrato_credito.txt", &[]);
        let c = find_by_amount(&without, "8000");
        assert!(c.included && c.kind == "income");
    }

    // T032
    #[test]
    fn transfer_to_the_holder_is_internal() {
        let cl = classified("banestes_extrato_credito.txt", &[]);
        let c = find_by_amount(&cl, "500");
        assert!(!c.included && c.reason == "interno", "{c:?}");
    }

    // T033
    #[test]
    fn categorizes_by_app_rules_with_outros_as_leftover() {
        let cl = classified("banestes_extrato_credito.txt", &[]);
        // "BETA SERVICOS MEDICOS LTDA" matches the MEDICO keyword
        assert_eq!(find_by_amount(&cl, "-128.47").category, "Saúde");
        // no rule matches, and Banestes ships no category of its own
        assert_eq!(find_by_amount(&cl, "-200").category, "Outros");
    }

    #[test]
    fn every_movement_of_the_real_statement_is_kept() {
        let cl = classified("banestes_extrato.txt", &["2026-07"]);
        assert_eq!(cl.len(), 9);
        assert!(cl.iter().all(|c| c.included), "nada a excluir neste extrato");
        assert!(cl.iter().all(|c| c.kind == "expense"));
        // deterministic, distinct ids
        let ids: std::collections::HashSet<_> = cl.iter().map(|c| c.id.clone()).collect();
        assert_eq!(ids.len(), 9);
    }

    #[test]
    fn formats_money_for_error_messages() {
        assert_eq!(format_brl(dec("10")), "10,00");
        assert_eq!(format_brl(dec("1234.5")), "1.234,50");
        assert_eq!(format_brl(dec("7106.11")), "7.106,11");
    }
}
