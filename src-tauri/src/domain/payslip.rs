use regex::Regex;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::OnceLock;
use uuid::Uuid;

/// One line of a payslip.
/// - `kind`: "rendimento" | "desconto"
/// - `class`: for rendimentos "salario" | "bonus" | "wash"; for descontos "recorrente" | "wash"
/// - `offsetting`: true when the line is cancelled by a mirror on the other side (advance reconciliation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayslipItem {
    pub kind: String,
    pub class: String,
    pub description: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    /// Net (líquido) attributable to this earning line, after proportional deductions.
    /// Zero for descontos and washes. Lets the UI show e.g. the net you get from the CD.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_share: Decimal,
    pub offsetting: bool,
}

/// A parsed monthly payslip (SouGov.br "Comprovante de Rendimentos").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payslip {
    pub id: Uuid,
    pub month: String, // ISO "YYYY-MM"
    #[serde(with = "rust_decimal::serde::str")]
    pub gross: Decimal, // as printed (may include wash)
    #[serde(with = "rust_decimal::serde::str")]
    pub real_gross: Decimal, // gross excluding offsetting washes
    #[serde(with = "rust_decimal::serde::str")]
    pub deductions: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net: Decimal, // líquido a receber (real cash)
    #[serde(with = "rust_decimal::serde::str")]
    pub salary_liq: Decimal, // net attributable to base salary
    #[serde(with = "rust_decimal::serde::str")]
    pub bonus_liq: Decimal, // net attributable to eventual bonus
    #[serde(with = "rust_decimal::serde::str")]
    pub ir_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fgts: Decimal,
    pub items: Vec<PayslipItem>,
    pub source_file: String,
    pub imported_at: String,
}

fn value_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\d{1,3}(?:\.\d{3})*,\d{2}").unwrap())
}
fn month_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?i)\b(JAN|FEV|MAR|ABR|MAI|JUN|JUL|AGO|SET|OUT|NOV|DEZ)\s+(\d{4})\b").unwrap()
    })
}

fn parse_money(s: &str) -> Option<Decimal> {
    Decimal::from_str(&s.replace('.', "").replace(',', ".")).ok()
}

fn month_num(abbr: &str) -> Option<u32> {
    match abbr.to_uppercase().as_str() {
        "JAN" => Some(1), "FEV" => Some(2), "MAR" => Some(3), "ABR" => Some(4),
        "MAI" => Some(5), "JUN" => Some(6), "JUL" => Some(7), "AGO" => Some(8),
        "SET" => Some(9), "OUT" => Some(10), "NOV" => Some(11), "DEZ" => Some(12),
        _ => None,
    }
}

/// Map a payslip deduction to an expense category, so it shows as a monthly cost.
pub fn deduction_category(desc: &str) -> String {
    let u = desc.to_uppercase();
    if u.contains("IMPOSTO") || u.contains("IRRF") || u.contains("RENDA") {
        "Impostos".into()
    } else if u.contains("GEAP") || u.contains("SAUDE") || u.contains("SAÚDE") || u.contains("PSAUDE") || u.contains("PSAÚDE") {
        "Saúde".into()
    } else if u.contains("FUNPRESP") || u.contains("SEGURIDADE") || u.contains("PSS") || u.contains("PREVID") {
        "Previdência".into()
    } else {
        "Descontos da folha".into()
    }
}

/// Normalize a description for wash matching: uppercase, drop trailing pure-digit
/// tokens (SouGov.br appends codes like "001"), collapse whitespace.
fn norm_desc(s: &str) -> String {
    let mut toks: Vec<&str> = s.split_whitespace().collect();
    while toks.last().map(|t| t.chars().all(|c| c.is_ascii_digit())).unwrap_or(false) {
        toks.pop();
    }
    toks.join(" ").to_uppercase()
}

/// Bonus / non-permanent earnings (checked first — may contain "ADICIONAL" etc.).
/// Includes "Cargo de Direção - CD": a temporary commissioned-post pay that ends
/// when the user leaves the post, so it must not anchor the permanent salary.
fn is_bonus(desc: &str) -> bool {
    let u = desc.to_uppercase();
    ["CARGO DE DIRE", "FUNCAO", "FUNÇÃO", "FG-", "FCPE",
     "FERIAS", "FÉRIAS", "NATALINA", "DECIMO", "DÉCIMO", "13", "ABONO", "RETROAT",
     "EXERC", "TERCO", "TERÇO", "DIFEREN", "GRATIF.NATAL", "ADIANT"]
        .iter()
        .any(|k| u.contains(k))
}
/// Parse the flattened text of a SouGov.br payslip into structured, classified data.
pub fn parse_payslip_text(text: &str, source_file: &str) -> Result<Payslip, String> {
    let month = parse_month(text).ok_or("Não achei o mês/ano no contracheque.")?;
    let mut items = parse_items(text)?;
    if items.is_empty() {
        return Err("Não achei rendimentos/descontos no contracheque.".into());
    }

    mark_washes(&mut items);
    classify(&mut items);

    let gross: Decimal = sum(&items, "rendimento", None);
    let real_gross: Decimal = items
        .iter()
        .filter(|i| i.kind == "rendimento" && !i.offsetting)
        .fold(dec!(0), |a, i| a + i.amount);
    let salary_gross: Decimal = items
        .iter()
        .filter(|i| i.kind == "rendimento" && i.class == "salario")
        .fold(dec!(0), |a, i| a + i.amount);
    let bonus_gross = (real_gross - salary_gross).max(dec!(0));

    let (_g, deductions, net, ir_base, fgts) = parse_totals(text)
        .unwrap_or((gross, sum(&items, "desconto", None), gross - sum(&items, "desconto", None), dec!(0), dec!(0)));
    let _ = bonus_gross;

    // Per-item net share: each non-wash earning's slice of the líquido, proportional to gross.
    if real_gross > dec!(0) {
        for it in items.iter_mut() {
            if it.kind == "rendimento" && !it.offsetting {
                it.net_share = (net * it.amount / real_gross).round_dp(2);
            }
        }
    }
    // salary = base permanent net; bonus = everything else (incl. temporary CD). Sum to net exactly.
    let salary_liq = items
        .iter()
        .filter(|i| i.kind == "rendimento" && i.class == "salario")
        .fold(dec!(0), |a, i| a + i.net_share);
    let salary_liq = salary_liq.min(net);
    let bonus_liq = (net - salary_liq).max(dec!(0));

    let id = Uuid::new_v5(&Uuid::NAMESPACE_URL, format!("payslip:{month}").as_bytes());

    Ok(Payslip {
        id,
        month,
        gross,
        real_gross,
        deductions,
        net,
        salary_liq,
        bonus_liq,
        ir_base,
        fgts,
        items,
        source_file: source_file.to_string(),
        imported_at: String::new(), // stamped by the caller
    })
}

fn sum(items: &[PayslipItem], kind: &str, class: Option<&str>) -> Decimal {
    items
        .iter()
        .filter(|i| i.kind == kind && class.map(|c| i.class == c).unwrap_or(true))
        .fold(dec!(0), |a, i| a + i.amount)
}

/// Mark rendimento/desconto mirror pairs (same normalized description + equal amount)
/// as offsetting washes — SouGov.br advance-reconciliation entries that net to zero.
fn mark_washes(items: &mut [PayslipItem]) {
    // Map each desconto (norm_desc, amount) to its index, consumed once when matched.
    let mut pool: HashMap<(String, String), Vec<usize>> = HashMap::new();
    for (i, it) in items.iter().enumerate() {
        if it.kind == "desconto" {
            pool.entry((norm_desc(&it.description), it.amount.to_string()))
                .or_default()
                .push(i);
        }
    }
    let mut wash_desc: Vec<usize> = Vec::new();
    let mut wash_rend: Vec<usize> = Vec::new();
    for (i, it) in items.iter().enumerate() {
        if it.kind != "rendimento" {
            continue;
        }
        let key = (norm_desc(&it.description), it.amount.to_string());
        if let Some(v) = pool.get_mut(&key) {
            if let Some(di) = v.pop() {
                wash_rend.push(i);
                wash_desc.push(di);
            }
        }
    }
    for i in wash_rend.into_iter().chain(wash_desc) {
        items[i].offsetting = true;
        items[i].class = "wash".to_string();
    }
}

fn classify(items: &mut [PayslipItem]) {
    for it in items.iter_mut() {
        if it.offsetting {
            it.class = "wash".to_string();
            continue;
        }
        it.class = match it.kind.as_str() {
            // Earnings are a bonus (CD, gratificações…) or, by default, base salary.
            "rendimento" if is_bonus(&it.description) => "bonus",
            "rendimento" => "salario",
            _ => "recorrente",
        }
        .to_string();
    }
}

fn parse_month(text: &str) -> Option<String> {
    let c = month_re().captures(text)?;
    let m = month_num(&c[1])?;
    let y: i32 = c[2].parse().ok()?;
    Some(format!("{y:04}-{m:02}"))
}

fn parse_items(text: &str) -> Result<Vec<PayslipItem>, String> {
    let line = text
        .lines()
        .find(|l| l.contains("RENDIMENTOS") && value_re().is_match(l))
        .ok_or("Linha de rendimentos não encontrada.")?;

    let matches: Vec<(Decimal, usize, usize)> = value_re()
        .find_iter(line)
        .filter_map(|m| parse_money(m.as_str()).map(|v| (v, m.start(), m.end())))
        .collect();

    let mut items = Vec::new();
    let mut kind = "rendimento";
    for i in 0..matches.len() {
        let (amount, _start, end) = matches[i];
        let desc_end = if i + 1 < matches.len() { matches[i + 1].1 } else { line.len() };
        let raw = &line[end..desc_end];
        if raw.contains("DESCONTOS") {
            kind = "desconto";
        }
        let desc = raw
            .replace("RENDIMENTOS", " ")
            .replace("DESCONTOS", " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        if desc.is_empty() {
            continue;
        }
        items.push(PayslipItem {
            kind: kind.to_string(),
            class: String::new(),
            description: desc,
            amount,
            net_share: dec!(0),
            offsetting: false,
        });
    }
    Ok(items)
}

/// Returns (gross, deductions, net, ir_base, fgts) from the totals line.
fn parse_totals(text: &str) -> Option<(Decimal, Decimal, Decimal, Decimal, Decimal)> {
    let lines: Vec<&str> = text.lines().collect();
    let hdr = lines.iter().position(|l| {
        let up = l.to_uppercase();
        up.contains("BRUTO") && (up.contains("LÍQUIDO") || up.contains("LIQUIDO"))
    })?;
    for l in lines.iter().skip(hdr + 1) {
        let vals: Vec<Decimal> = value_re().find_iter(l).filter_map(|m| parse_money(m.as_str())).collect();
        if vals.len() >= 4 {
            let net = *vals.last().unwrap();
            let gross = vals.iter().cloned().max().unwrap_or(net);
            let deductions = gross - net;
            let ir_base = if vals.len() == 6 { vals[4] } else { dec!(0) };
            let fgts = if vals.len() == 6 { vals[1] } else { dec!(0) };
            return Some((gross, deductions, net, ir_base, fgts));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // Clean month (no advances): only base salary.
    const CLEAN: &str = "\
NORMAL
JUN 2026
TIPO  DISCRIMINAÇÃO  PRAZO  VALOR
9.616,10VENCIMENTO BASICORENDIMENTOS 1.192,00AUXÍLIO-ALIMENTACÃO 4.989,43CARGO DE DIRECAO - CD 11.058,51RT - RSC LEI 12.772/12 AT 1.036,92FUNPRESP-CONTR.MENSAL NORMALDESCONTOS 1.140,14PSAúDE AUTOGESTãO - GEAP 988,07CONT. PLANO SEGURIDADE SOCIAL 5.592,00IMPOSTO DE RENDA RETIDO FONTE
BASE CÁLCULO DO TETO DEPÓSITO FGTS BRUTO DESCONTOBASE CÁLCULO DO I.R. LÍQUIDO
0,00 0,00 26.856,04 8.757,1323.639,05 18.098,91
";

    // Month with an advance (13º/férias) mirrored on both sides → wash, net zero.
    const WASH: &str = "\
JAN 2026
TIPO  DISCRIMINAÇÃO  PRAZO  VALOR
9.190,03VENCIMENTO BASICORENDIMENTOS 1.175,00AUXÍLIO-ALIMENTACÃO 001 11.946,52ADIANT.GRATIF.NATALINA AT 001 4.989,43CARGO DE DIRECAO - CD 10.568,54RT - RSC LEI 12.772/12 AT 001 11.946,52ADIANT.GRATIF.NATALINA AT 001DESCONTOS 959,05FUNPRESP-CONTR.MENSAL NORMAL 988,07CONT. PLANO SEGURIDADE SOCIAL
BRUTO DESCONTO LÍQUIDO
40.815,04 23.224,04 17.591,00
";

    #[test]
    fn clean_month_splits_salary_and_cd_bonus() {
        let p = parse_payslip_text(CLEAN, "f.pdf").unwrap();
        assert_eq!(p.month, "2026-06");
        assert_eq!(p.net, dec!(18098.91));
        assert_eq!(p.real_gross, dec!(26856.04));
        assert!(p.items.iter().all(|i| i.class != "wash"));
        // CD is a temporary bonus, not base salary.
        let cd = p.items.iter().find(|i| i.description.contains("CARGO DE DIRE")).unwrap();
        assert_eq!(cd.class, "bonus");
        // Net you actually get from the CD ≈ R$ 3.36k (proportional share of the líquido).
        assert!(cd.net_share > dec!(3300) && cd.net_share < dec!(3420), "cd net_share = {}", cd.net_share);
        // salary + bonus = net exactly; the only bonus this month is the CD.
        assert_eq!(p.salary_liq + p.bonus_liq, p.net);
        assert_eq!(p.bonus_liq, cd.net_share);
        assert!(p.salary_liq < p.net);
    }

    #[test]
    fn advance_is_washed_out() {
        let p = parse_payslip_text(WASH, "f.pdf").unwrap();
        // The 11.946,52 advance appears as rendimento AND desconto → both marked wash.
        let washes: Vec<_> = p.items.iter().filter(|i| i.offsetting).collect();
        assert_eq!(washes.len(), 2, "one rendimento + one desconto washed");
        assert!(washes.iter().all(|i| i.class == "wash"));
        // real gross excludes the wash: 9190.03+1175+4989.43+10568.54 = 25923.00
        assert_eq!(p.real_gross, dec!(25923.00));
        // CD present → bonus > 0; salary + bonus = net.
        assert!(p.bonus_liq > dec!(0));
        assert_eq!(p.salary_liq + p.bonus_liq, p.net);
    }

    #[test]
    fn deterministic_id_per_month() {
        let a = parse_payslip_text(CLEAN, "a.pdf").unwrap();
        let b = parse_payslip_text(CLEAN, "b.pdf").unwrap();
        assert_eq!(a.id, b.id);
    }
}
