//! Domain: Santander card-invoice parsing (pure).
//!
//! [`FaturaSantander`] is the typed picture of one invoice PDF's text: metadata, the
//! declared "Resumo da Fatura" totals, and every movement of every card subsection
//! (physical + virtual cards share one invoice). Pipeline mirrors
//! `banestes_statement` (014): `parse` (text → struct), `conferir` (integrity checks
//! → [`Conferencia`]), `into_transactions` (→ the shared [`Transaction`] model the
//! BTG invoice reader also produces).
//!
//! Grammar highlights (see `specs/015-santander-invoice-adapter/research.md` R2):
//!
//! ```text
//! Detalhamento da Fatura
//! FULANO F TAL -  4111 XXXX XXXX 1111          ← card subsection
//! Pagamento e Demais Créditos                  ← credits block
//!   03/06 PAGAMENTO DE FATURA-INTERNET -4.923,40   ← excluded (transfer)
//!   29/06 DESCONTO DO MES -149,30                  ← cashback → credit
//! Despesas                                     ← expenses block
//!   03/06 ACME* TEAM T1 7.019,45 1.320,29          ← international: R$ + US$ (US$ ignored)
//! COTAÇÃO DOLAR R$ 5,3166                          ← dropped
//! IOF DESPESA NO EXTERIOR 245,68                   ← own transaction (else totals break)
//! Resumo da Fatura                             ← reconciliation block
//! ```

use std::sync::OnceLock;

use chrono::NaiveDate;
use regex::Regex;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::bank_statement::{norm, parse_amount};
use super::categorizer::Categorizer;
use super::invoice::YearMonth;
use super::transaction::{InstallmentInfo, ParseWarning, Transaction};

/// Movement line: optional "Compra" column number, day/month, description,
/// optional installment (dd/dd), R$ value (sign attached), optional US$ value.
fn re_movement() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"^(?:\d+\s+)?(\d{2})/(\d{2})\s+(.+?)(?:\s+(\d{2}/\d{2}))?\s+(-?\d{1,3}(?:\.\d{3})*,\d{2})(?:\s+(-?\d{1,3}(?:\.\d{3})*,\d{2}))?\s*$",
        )
        .unwrap()
    })
}
/// `dd/mm/yyyy` anywhere in a line (Vencimento).
fn re_full_date() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\d{2})/(\d{2})/(\d{4})").unwrap())
}
/// First money value in a line.
fn re_money() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"-?\d{1,3}(?:\.\d{3})*,\d{2}").unwrap())
}
/// `Fatura_MMYYYY` in the filename.
fn re_filename_month() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)fatura_(\d{2})(\d{4})").unwrap())
}
/// Card subsection header: masked card number.
fn re_card_header() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"X{4} X{4} \d{4}\s*$").unwrap())
}

/// True when this text looks like a Santander card invoice. A SouGov payslip or a
/// Banestes statement matches neither marker pair.
pub fn is_santander_invoice(text: &str) -> bool {
    let n = norm(text);
    n.contains("DETALHAMENTO DA FATURA")
        && (n.contains("RESUMO DA FATURA") || n.contains("BANCO SANTANDER"))
}

/// One movement read from the Detalhamento (a purchase, an IOF line, or a credit).
/// Day/month only — the year is resolved against the due date (research R5) when the
/// struct becomes transactions.
#[derive(Debug, Clone)]
pub struct Compra {
    pub day: u32,
    pub month: u32,
    pub description: String,
    pub amount: Decimal,
    pub installment: Option<InstallmentInfo>,
}

/// The totals the PDF declares in "Resumo da Fatura" — the reconciliation source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumoFatura {
    pub saldo_anterior: Decimal,
    pub despesas_brasil: Decimal,
    pub despesas_exterior: Decimal,
    pub pagamentos: Decimal,
    pub creditos: Decimal,
    pub saldo_fatura: Decimal,
}

/// Outcome of one integrity check (same semantics as the 014 statement checks).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checagem {
    Fechou,
    Divergiu { diferenca: Decimal },
    SemDados { faltou: &'static str },
}

/// The two independent checks a Santander invoice allows (research R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conferencia {
    /// Σ despesas lidas == despesas Brasil + Exterior declaradas.
    pub despesas: Checagem,
    /// Σ |créditos lidos| + pagamentos excluídos == créditos + pagamentos declarados.
    pub creditos_pagamentos: Checagem,
}

impl Conferencia {
    /// Strict policy (014): anything other than `Fechou` on both checks is an error —
    /// a lost line must become a visible refusal, never a silently cheaper month.
    pub fn exigir(&self) -> Result<(), String> {
        let checks = [
            (&self.despesas, "com as despesas declaradas"),
            (&self.creditos_pagamentos, "com os pagamentos e créditos declarados"),
        ];
        for (checagem, o_que) in checks {
            match checagem {
                Checagem::Fechou => {}
                Checagem::Divergiu { diferenca } => {
                    return Err(format!(
                        "A leitura da fatura não fechou {o_que} (diferença de R$ {}). \
                         Nada foi importado.",
                        format_brl(diferenca.abs())
                    ))
                }
                Checagem::SemDados { faltou } => {
                    return Err(format!(
                        "Não encontrei {faltou} nesta fatura para conferir a leitura. \
                         Nada foi importado."
                    ))
                }
            }
        }
        Ok(())
    }
}

/// Typed picture of one Santander invoice, before it becomes `Invoice`/`Transaction`s.
#[derive(Debug, Default)]
pub struct FaturaSantander {
    pub titular: String,
    pub vencimento: Option<NaiveDate>,
    /// Purchases + IOF lines (positive) and real credits like cashback (negative),
    /// in PDF order across every card subsection.
    pub movimentos: Vec<Compra>,
    /// Σ of the excluded invoice-payment lines (transfers, never transactions).
    pub pagamentos_excluidos: Decimal,
    pub resumo: Option<ResumoFatura>,
}

impl FaturaSantander {
    /// Parse the text of a Santander invoice PDF. Structure only — call
    /// [`Self::conferir`] before trusting the movements.
    pub fn parse(text: &str) -> Result<Self, String> {
        if !is_santander_invoice(text) {
            return Err("Este PDF não é uma fatura do Santander.".into());
        }

        let mut this = FaturaSantander::default();
        let lines: Vec<&str> = text.lines().collect();

        // Metadata scan (whole document): holder = first card-subsection-looking
        // line; due date = first "Vencimento" with a full date on it or the next line.
        for (i, raw) in lines.iter().enumerate() {
            let line = raw.trim();
            if this.titular.is_empty() && re_card_header().is_match(line) {
                if let Some((name, _)) = line.split_once(" - ") {
                    this.titular = name.trim().to_string();
                }
            }
            if this.vencimento.is_none() && norm(line).contains("VENCIMENTO") {
                let candidate = re_full_date()
                    .captures(line)
                    .or_else(|| lines.get(i + 1).and_then(|n| re_full_date().captures(n)));
                if let Some(c) = candidate {
                    this.vencimento = NaiveDate::from_ymd_opt(
                        c[3].parse().unwrap_or(0),
                        c[2].parse().unwrap_or(0),
                        c[1].parse().unwrap_or(0),
                    );
                }
            }
        }
        if this.vencimento.is_none() {
            return Err("Não reconheci o formato desta fatura Santander (sem vencimento).".into());
        }

        let start = lines
            .iter()
            .position(|l| norm(l).contains("DETALHAMENTO DA FATURA"))
            .ok_or("Não reconheci o formato desta fatura Santander.")?;

        #[derive(PartialEq)]
        enum Bloco {
            Creditos,
            Despesas,
        }
        let mut bloco = Bloco::Despesas;
        let mut resumo_lines: Vec<&str> = Vec::new();
        let mut in_resumo = false;

        for raw in &lines[start + 1..] {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let n = norm(line);

            if n.starts_with("RESUMO DA FATURA") {
                in_resumo = true;
                continue;
            }
            if in_resumo {
                // The summary block ends at the next section heading; collecting a
                // few extra label-less lines is harmless (labels drive the parse).
                if n.starts_with("SALDO TOTAL CONSOLIDADO") || n.starts_with("JUROS E CUSTO") {
                    in_resumo = false;
                    continue;
                }
                resumo_lines.push(line);
                continue;
            }

            // Section state.
            if re_card_header().is_match(line) {
                bloco = Bloco::Despesas;
                continue;
            }
            if n.starts_with("PAGAMENTO E DEMAIS CREDITOS") {
                bloco = Bloco::Creditos;
                continue;
            }
            if n == "DESPESAS" {
                bloco = Bloco::Despesas;
                continue;
            }
            // Noise inside the table.
            if n.starts_with("COMPRA DATA DESCRICAO")
                || n.starts_with("VALOR TOTAL")
                || n.starts_with("COTACAO DOLAR")
            {
                continue;
            }
            // IOF of the previous international purchase → its own movement, same
            // date, description pointing back at the purchase (research R3).
            if n.starts_with("IOF DESPESA NO EXTERIOR") {
                let Some(valor) = re_money().find(line).and_then(|m| parse_amount(m.as_str()))
                else {
                    continue;
                };
                let Some(prev) = this.movimentos.last() else { continue };
                let (day, month, desc) = (prev.day, prev.month, prev.description.clone());
                this.movimentos.push(Compra {
                    day,
                    month,
                    description: format!("IOF — {desc}"),
                    amount: valor,
                    installment: None,
                });
                continue;
            }

            let Some(c) = re_movement().captures(line) else { continue };
            let day: u32 = c[1].parse().unwrap_or(0);
            let month: u32 = c[2].parse().unwrap_or(0);
            let description = c[3].trim().to_string();
            let installment = c.get(4).and_then(|m| {
                let (cur, tot) = m.as_str().split_once('/')?;
                Some(InstallmentInfo { current: cur.parse().ok()?, total: tot.parse().ok()? })
            });
            let Some(amount) = parse_amount(&c[5]) else { continue };

            // Invoice payments are bank transfers already visible (and excluded) on
            // the statement side — counting them here would double them (research R4).
            let ndesc = norm(&description);
            if bloco == Bloco::Creditos
                && (ndesc.contains("PAGAMENTO DE FATURA") || ndesc.contains("DEB AUTOM DE FATURA"))
            {
                this.pagamentos_excluidos += amount.abs();
                continue;
            }
            if amount.is_zero() {
                continue; // FR-006: an exempt annuity is not a transaction.
            }
            this.movimentos.push(Compra { day, month, description, amount, installment });
        }

        this.resumo = parse_resumo(&resumo_lines);
        Ok(this)
    }

    /// Run both integrity checks against the declared "Resumo da Fatura".
    pub fn conferir(&self) -> Conferencia {
        let Some(resumo) = &self.resumo else {
            return Conferencia {
                despesas: Checagem::SemDados { faltou: "o resumo da fatura" },
                creditos_pagamentos: Checagem::SemDados { faltou: "o resumo da fatura" },
            };
        };

        // Aggregate on purpose: the PDF counts each IOF in the *Brasil* total while
        // the purchase itself sits in *Exterior* — summing both sides declares the
        // same money without classifying lines per column (research R7).
        let lidas: Decimal =
            self.movimentos.iter().map(|m| m.amount).filter(|a| a.is_sign_positive()).sum();
        let declaradas = resumo.despesas_brasil + resumo.despesas_exterior;
        let despesas = if lidas == declaradas {
            Checagem::Fechou
        } else {
            Checagem::Divergiu { diferenca: lidas - declaradas }
        };

        let creditos_lidos: Decimal = self
            .movimentos
            .iter()
            .map(|m| m.amount)
            .filter(|a| a.is_sign_negative())
            .map(|a| a.abs())
            .sum();
        let lado_lido = creditos_lidos + self.pagamentos_excluidos;
        let lado_declarado = resumo.creditos + resumo.pagamentos;
        let creditos_pagamentos = if lado_lido == lado_declarado {
            Checagem::Fechou
        } else {
            Checagem::Divergiu { diferenca: lado_lido - lado_declarado }
        };

        Conferencia { despesas, creditos_pagamentos }
    }

    /// Reference month: `Fatura_MMYYYY` from the filename; fallback = due date
    /// month (research R10).
    pub fn reference_month(&self, filename: &str) -> YearMonth {
        if let Some(c) = re_filename_month().captures(filename) {
            let (m, y): (u8, i32) = (c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0));
            if (1..=12).contains(&m) && y >= 2000 {
                return YearMonth::new(y, m);
            }
        }
        let v = self.vencimento.expect("parse garante vencimento");
        YearMonth::from_date(v)
    }

    /// Movements → categorized transactions in PDF order (sequential row_index →
    /// deterministic ids, research R6). Purchase years resolve against the due date:
    /// a purchase month greater than the due month belongs to the previous year
    /// (dec→jan rollover, research R5).
    pub fn into_transactions(
        self,
        invoice_id: Uuid,
        categorizer: &Categorizer,
    ) -> (Vec<Transaction>, Vec<ParseWarning>) {
        let venc = self.vencimento.expect("parse garante vencimento");
        let (venc_year, venc_month) = {
            use chrono::Datelike;
            (venc.year(), venc.month())
        };
        let mut txs = Vec::new();
        let mut warnings = Vec::new();

        for (i, m) in self.movimentos.into_iter().enumerate() {
            let year = if m.month > venc_month { venc_year - 1 } else { venc_year };
            let Some(date) = NaiveDate::from_ymd_opt(year, m.month, m.day) else {
                warnings.push(ParseWarning {
                    row: i as u32,
                    message: format!("data inválida em '{}' — linha ignorada", m.description),
                });
                continue;
            };
            let category = categorizer.categorize(&m.description);
            txs.push(Transaction::new(
                invoice_id,
                i as u32,
                date,
                m.description,
                m.amount,
                category,
                m.installment,
            ));
        }
        (txs, warnings)
    }
}

/// Extract the declared totals from the collected "Resumo da Fatura" lines. The
/// exterior line carries two money values (R$ + US$) — the first (R$) is the one the
/// reconciliation uses.
fn parse_resumo(lines: &[&str]) -> Option<ResumoFatura> {
    let mut saldo_anterior = None;
    let mut brasil = None;
    let mut exterior = None;
    let mut pagamentos = None;
    let mut creditos = None;
    let mut saldo_fatura = None;

    for line in lines {
        let n = norm(line);
        let value = || re_money().find(line).and_then(|m| parse_amount(m.as_str()));
        if n.starts_with("SALDO ANTERIOR") {
            saldo_anterior = value();
        } else if n.contains("DESPESAS/DEBITOS NO BRASIL") {
            brasil = value();
        } else if n.contains("DESPESAS/DEBITOS NO EXTERIOR") {
            exterior = value();
        } else if n.contains("TOTAL DE PAGAMENTOS") {
            pagamentos = value().map(|v| v.abs());
        } else if n.contains("TOTAL DE CREDITOS") {
            creditos = value().map(|v| v.abs());
        } else if n.contains("SALDO DESTA FATURA") {
            saldo_fatura = value();
        }
    }

    let resumo = ResumoFatura {
        saldo_anterior: saldo_anterior?,
        despesas_brasil: brasil?,
        despesas_exterior: exterior?,
        pagamentos: pagamentos?,
        creditos: creditos?,
        saldo_fatura: saldo_fatura?,
    };
    // The block must be internally coherent, or it cannot be trusted as a net.
    let identidade = resumo.saldo_anterior + resumo.despesas_brasil + resumo.despesas_exterior
        - resumo.pagamentos
        - resumo.creditos;
    (identidade == resumo.saldo_fatura).then_some(resumo)
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

#[cfg(test)]
mod tests {
    use super::*;
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
        fixture("santander_fatura.txt")
    }

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn inv_id() -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_URL, b"Fatura_072026_TESTE_SANTANDER.PDF")
    }

    const PAYSLIP_TEXT: &str = "\
Comprovante de Rendimentos
SouGov.br
JUL 2026
Vencimento Base 12.345,67
Líquido a Receber 10.345,67";

    // T007
    #[test]
    fn detects_santander_and_rejects_other_documents() {
        assert!(is_santander_invoice(&main_fixture()));
        assert!(!is_santander_invoice(PAYSLIP_TEXT));
        assert!(!is_santander_invoice(&fixture("banestes_extrato.txt")));
        assert!(!is_santander_invoice(""));
    }

    // T027 — foreign documents are refused with the user-facing message (it reaches
    // the UI through mapError, which requires an accented Portuguese sentence).
    #[test]
    fn parse_refuses_payslip_and_bank_statement_texts() {
        for text in [PAYSLIP_TEXT.to_string(), fixture("banestes_extrato.txt")] {
            let err = FaturaSantander::parse(&text).unwrap_err();
            assert!(err.contains("não é uma fatura do Santander"), "{err}");
        }
    }

    // T008
    #[test]
    fn parses_every_card_subsection_and_drops_noise() {
        let f = FaturaSantander::parse(&main_fixture()).unwrap();
        assert_eq!(f.titular, "MARIA APARECIDA DA SILVA SOUZA");
        assert_eq!(f.vencimento, NaiveDate::from_ymd_opt(2026, 7, 5));

        // 8 purchases + 6 IOFs from card 2222, 1 cashback credit from card 1111.
        // The zero-value ANUIDADE and every noise line must be out.
        let positivos = f.movimentos.iter().filter(|m| m.amount > Decimal::ZERO).count();
        assert_eq!(positivos, 14, "8 compras + 6 IOFs: {:#?}", f.movimentos);
        for m in &f.movimentos {
            let d = m.description.to_uppercase();
            assert!(!m.amount.is_zero(), "transação de valor zero: {d}");
            assert!(!d.contains("COTAÇÃO") && !d.contains("COTACAO"), "cotação virou compra: {d}");
            assert!(!d.contains("VALOR TOTAL"), "total de seção virou compra: {d}");
            assert!(!d.starts_with("COMPRA DATA"), "cabeçalho virou compra: {d}");
            assert!(!d.contains("ANUIDADE"), "anuidade 0,00 virou transação");
        }
        // Purchases from the second card subsection are present.
        assert!(f.movimentos.iter().any(|m| m.description == "NUVEM BRASIL LTDA"));
    }

    // T009
    #[test]
    fn international_purchase_uses_brl_and_gets_its_own_iof_line() {
        let f = FaturaSantander::parse(&main_fixture()).unwrap();
        let compra = f
            .movimentos
            .iter()
            .find(|m| m.description == "ACME* TEAM T1")
            .expect("compra internacional");
        assert_eq!(compra.amount, dec("7019.45"), "R$ impresso, não o US$ (1.320,29)");
        assert_eq!((compra.day, compra.month), (3, 6));

        let iof = f
            .movimentos
            .iter()
            .find(|m| m.description == "IOF — ACME* TEAM T1")
            .expect("IOF da compra internacional");
        assert_eq!(iof.amount, dec("245.68"));
        assert_eq!((iof.day, iof.month), (3, 6), "IOF herda a data da compra");

        // Every international purchase got exactly one IOF partner.
        let iofs = f.movimentos.iter().filter(|m| m.description.starts_with("IOF — ")).count();
        assert_eq!(iofs, 6);
    }

    // T010
    #[test]
    fn invoice_payments_are_excluded_and_cashback_is_a_credit() {
        let f = FaturaSantander::parse(&main_fixture()).unwrap();
        assert!(
            !f.movimentos.iter().any(|m| m.description.contains("PAGAMENTO DE FATURA")),
            "pagamento de fatura virou transação"
        );
        assert_eq!(f.pagamentos_excluidos, dec("21005.57"));

        let cashback = f
            .movimentos
            .iter()
            .find(|m| m.description == "DESCONTO DO MES")
            .expect("cashback presente");
        assert_eq!(cashback.amount, dec("-149.30"));

        // Through into_transactions the negative becomes an explicit reversal.
        let (txs, _) = f.into_transactions(inv_id(), &Categorizer::with_defaults());
        let tx = txs.iter().find(|t| t.description == "DESCONTO DO MES").unwrap();
        assert!(tx.is_reversal);
    }

    // T010 (grafia DEB AUTOM) + T011 (virada dez→jan)
    #[test]
    fn cashback_fixture_handles_deb_autom_and_year_rollover() {
        let f = FaturaSantander::parse(&fixture("santander_fatura_cashback.txt")).unwrap();
        assert_eq!(f.pagamentos_excluidos, dec("1000.00"));
        assert_eq!(f.vencimento, NaiveDate::from_ymd_opt(2027, 1, 5));

        let (txs, warnings) = f.into_transactions(inv_id(), &Categorizer::with_defaults());
        assert!(warnings.is_empty(), "{warnings:?}");
        let dez = txs.iter().find(|t| t.description == "MERCADO BOM PRECO").unwrap();
        assert_eq!(dez.date, NaiveDate::from_ymd_opt(2026, 12, 20).unwrap(), "dez fica no ano anterior");
        let jan = txs.iter().find(|t| t.description == "TAXI LEGAL").unwrap();
        assert_eq!(jan.date, NaiveDate::from_ymd_opt(2027, 1, 2).unwrap(), "jan fica no ano do vencimento");
    }

    // T011 (ano normal)
    #[test]
    fn purchase_year_comes_from_the_due_date() {
        let f = FaturaSantander::parse(&main_fixture()).unwrap();
        let (txs, warnings) = f.into_transactions(inv_id(), &Categorizer::with_defaults());
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(txs
            .iter()
            .all(|t| t.date >= NaiveDate::from_ymd_opt(2026, 5, 29).unwrap()
                && t.date <= NaiveDate::from_ymd_opt(2026, 6, 29).unwrap()));
    }

    // T012
    #[test]
    fn reference_month_from_filename_with_due_date_fallback() {
        let f = FaturaSantander::parse(&main_fixture()).unwrap();
        assert_eq!(
            f.reference_month("Fatura_072026_MARIA_1111_VISA_000_SANTANDER.PDF"),
            YearMonth::new(2026, 7)
        );
        // Renamed file → falls back to the printed due date (05/07/2026).
        assert_eq!(f.reference_month("fatura-renomeada.pdf"), YearMonth::new(2026, 7));
    }

    // ---- US2: a fatura confere ou não entra ----

    // T023
    #[test]
    fn both_checks_close_on_intact_fixtures() {
        for name in ["santander_fatura.txt", "santander_fatura_cashback.txt"] {
            let f = FaturaSantander::parse(&fixture(name)).unwrap();
            let c = f.conferir();
            assert_eq!(c.despesas, Checagem::Fechou, "{name}");
            assert_eq!(c.creditos_pagamentos, Checagem::Fechou, "{name}");
            assert!(c.exigir().is_ok(), "{name}");
        }
    }

    // T023 — the declared summary itself must be internally coherent to be trusted.
    #[test]
    fn incoherent_resumo_is_rejected_as_missing() {
        let text = main_fixture().replace("(=) Saldo Desta Fatura 2.806,82", "(=) Saldo Desta Fatura 9.999,99");
        let f = FaturaSantander::parse(&text).unwrap();
        assert!(f.resumo.is_none(), "resumo incoerente não pode virar rede de conferência");
        assert!(matches!(f.conferir().despesas, Checagem::SemDados { .. }));
    }

    // T024 — a tampered value must be refused with the exact difference.
    #[test]
    fn tampered_fixture_reports_the_difference_and_imports_nothing() {
        let f = FaturaSantander::parse(&fixture("santander_fatura_quebrada.txt")).unwrap();
        let c = f.conferir();
        assert_eq!(c.despesas, Checagem::Divergiu { diferenca: dec("100.00") });
        let err = c.exigir().unwrap_err();
        assert!(
            err.contains("não fechou") && err.contains("100,00") && err.contains("Nada foi importado"),
            "{err}"
        );
    }

    // T024 — no summary block ⇒ refuse saying what was missing.
    #[test]
    fn missing_resumo_refuses_with_a_clear_message() {
        let text = main_fixture().replace("Resumo da Fatura", "Bloco Removido");
        let f = FaturaSantander::parse(&text).unwrap();
        let err = f.conferir().exigir().unwrap_err();
        assert!(err.contains("resumo da fatura") && err.contains("Nada foi importado"), "{err}");
    }

    // T013
    #[test]
    fn transactions_have_deterministic_ids_and_app_categories() {
        let cz = Categorizer::with_defaults();
        let a = FaturaSantander::parse(&main_fixture()).unwrap().into_transactions(inv_id(), &cz);
        let b = FaturaSantander::parse(&main_fixture()).unwrap().into_transactions(inv_id(), &cz);
        let ids_a: Vec<_> = a.0.iter().map(|t| t.id).collect();
        let ids_b: Vec<_> = b.0.iter().map(|t| t.id).collect();
        assert_eq!(ids_a, ids_b, "mesma fixture ⇒ mesmos ids");
        assert_eq!(ids_a.len(), 15, "14 despesas + 1 cashback");
        let unicos: std::collections::HashSet<_> = ids_a.iter().collect();
        assert_eq!(unicos.len(), ids_a.len(), "ids distintos");

        // No Santander-provided category: everything comes from the app's rules;
        // whatever no rule matches lands in "Outros".
        assert!(a.0.iter().any(|t| t.category == "Outros"));
    }
}
