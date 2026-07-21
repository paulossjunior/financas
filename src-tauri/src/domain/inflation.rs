//! Inflation indicators: official IPCA + a personalized inflation rate that
//! reweights the IPCA group variations by the user's own spending shares.
//!
//! Pure domain logic (no I/O). The literature (ECB/BIS/BBVA/NBER) shows the
//! main driver of household inflation heterogeneity is the consumption basket,
//! which is exactly what reweighting by category shares captures.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// Monthly variation of one IPCA group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcaGroup {
    pub name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub month_var: Decimal,
}

/// IPCA monthly variation for one month (time series point).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcaPoint {
    pub month: String, // "YYYY-MM"
    #[serde(with = "rust_decimal::serde::str")]
    pub value: Decimal,
}

/// Official IPCA headline for the latest period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcaHeadline {
    pub ref_month: String, // "YYYY-MM"
    #[serde(with = "rust_decimal::serde::str")]
    pub month: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub year: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub twelve: Decimal,
}

/// Locally-cached index snapshot (persisted as JSON).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationCache {
    pub headline: IpcaHeadline,
    pub groups: Vec<IpcaGroup>,
    #[serde(default)]
    pub series: Vec<IpcaPoint>,
    pub fetched_at: String,
}

/// DTO returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationData {
    pub available: bool,
    pub headline: Option<IpcaHeadline>,
    pub groups: Vec<IpcaGroup>,
    pub series: Vec<IpcaPoint>,
    #[serde(with = "rust_decimal::serde::str")]
    pub personal_month: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub personal_diff: Decimal, // personal_month − headline.month (p.p.)
    pub fetched_at: String,
}

impl InflationData {
    pub fn empty() -> Self {
        Self {
            available: false,
            headline: None,
            groups: Vec::new(),
            series: Vec::new(),
            personal_month: dec!(0),
            personal_diff: dec!(0),
            fetched_at: String::new(),
        }
    }
}

/// The 9 official IPCA groups (canonical names, as stored from the IBGE fetch).
pub const IPCA_GROUPS: [&str; 9] = [
    "Alimentação e bebidas",
    "Habitação",
    "Artigos de residência",
    "Vestuário",
    "Transportes",
    "Saúde e cuidados pessoais",
    "Despesas pessoais",
    "Educação",
    "Comunicação",
];

fn up(s: &str) -> String {
    s.to_uppercase()
}

/// Map an app category name to an IPCA group (canonical name), or None to fall
/// back to the general index.
pub fn map_category_to_group(category: &str) -> Option<&'static str> {
    let u = up(category);
    let has = |ks: &[&str]| ks.iter().any(|k| u.contains(k));
    if has(&["ALIMENT", "LANCHE", "ALMOC", "ALMOÇ", "MERCADO", "SUPERMERCAD", "CERVEJA", "BAR", "RESTAURA", "PADAR", "HORTIFRUTI", "ACOUGUE", "AÇOUGUE", "DELIVERY", "IFOOD"]) {
        Some("Alimentação e bebidas")
    } else if has(&["MORAD", "ALUGUEL", "ENERGIA", "LUZ", "AGUA", "ÁGUA", "INTERNET", "CONDOMIN", "GAS", "GÁS", "FAXIN", "SANEAM"]) {
        Some("Habitação")
    } else if has(&["TRANSPORT", "COMBUST", "CARRO", "UBER", "ONIBUS", "ÔNIBUS", "GASOLINA", "ESTACIONAMENTO", "PEDAGIO", "PEDÁGIO", "99"]) {
        Some("Transportes")
    } else if has(&["SAUDE", "SAÚDE", "FARMAC", "FARMÁC", "REMEDIO", "REMÉDIO", "TERAPIA", "PSICOL", "MEDIC", "DENTIST", "PLANO", "HOSPITAL", "EXAME"]) {
        Some("Saúde e cuidados pessoais")
    } else if has(&["EDUCA", "CURSO", "ESCOLA", "FACULD", "MENSALIDADE", "LIVRO"]) {
        Some("Educação")
    } else if has(&["ASSINAT", "SERVIÇOS DE TI", "SERVICOS DE TI", "TELEFON", "CELULAR", "STREAMING", "TI", "SOFTWARE", "COMUNICA"]) {
        Some("Comunicação")
    } else if has(&["VESTU", "ROUPA", "CALCAD", "CALÇAD", "MODA"]) {
        Some("Vestuário")
    } else if has(&["RESIDENCIA", "RESIDÊNCIA", "MOVEL", "MÓVEL", "ELETRO", "MAQUINA", "MÁQUINA", "UTENSIL"]) {
        Some("Artigos de residência")
    } else if has(&["LAZER", "VIAGEM", "CACHORR", "PET", "COMPRAS ONLINE", "ENTRETEN", "BELEZA", "SALAO", "SALÃO", "PRESENTE"]) {
        Some("Despesas pessoais")
    } else {
        None
    }
}

/// Personal monthly inflation = spend-weighted average of the mapped group
/// variations (unmapped categories use the general index). Returns
/// (personal_month, diff_vs_general). With no spending it equals the general index.
pub fn compute_personal_inflation(
    categories: &[(String, Decimal)],
    groups: &[IpcaGroup],
    general_month: Decimal,
) -> (Decimal, Decimal) {
    let total: Decimal = categories.iter().fold(dec!(0), |a, (_, amt)| a + *amt);
    if total <= dec!(0) {
        return (general_month, dec!(0));
    }
    let group_var = |name: &str| -> Decimal {
        groups
            .iter()
            .find(|g| g.name == name)
            .map(|g| g.month_var)
            .unwrap_or(general_month)
    };
    let weighted: Decimal = categories.iter().fold(dec!(0), |a, (cat, amt)| {
        let var = match map_category_to_group(cat) {
            Some(gname) => group_var(gname),
            None => general_month,
        };
        a + *amt * var
    });
    let personal = (weighted / total).round_dp(2);
    (personal, personal - general_month)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<IpcaGroup> {
        vec![
            IpcaGroup { name: "Alimentação e bebidas".into(), month_var: dec!(-0.24) },
            IpcaGroup { name: "Habitação".into(), month_var: dec!(0.63) },
            IpcaGroup { name: "Transportes".into(), month_var: dec!(0.17) },
            IpcaGroup { name: "Saúde e cuidados pessoais".into(), month_var: dec!(0.23) },
        ]
    }

    #[test]
    fn maps_categories_to_groups() {
        assert_eq!(map_category_to_group("Alimentação"), Some("Alimentação e bebidas"));
        assert_eq!(map_category_to_group("Lanche"), Some("Alimentação e bebidas"));
        assert_eq!(map_category_to_group("Transporte"), Some("Transportes"));
        assert_eq!(map_category_to_group("Combustivel"), Some("Transportes"));
        assert_eq!(map_category_to_group("Moradia & Serviços"), Some("Habitação"));
        assert_eq!(map_category_to_group("Saúde"), Some("Saúde e cuidados pessoais"));
        assert_eq!(map_category_to_group("Assinaturas"), Some("Comunicação"));
        assert_eq!(map_category_to_group("Cachorros"), Some("Despesas pessoais"));
        assert_eq!(map_category_to_group("XYZ Desconhecido"), None);
    }

    #[test]
    fn reweights_by_spending() {
        // 70% Alimentação (-0.24) + 30% Transporte (0.17) = -0.168+0.051 = -0.117 → -0.12
        let cats = vec![("Alimentação".into(), dec!(700)), ("Transporte".into(), dec!(300))];
        let (personal, diff) = compute_personal_inflation(&cats, &groups(), dec!(0.16));
        assert_eq!(personal, dec!(-0.12));
        assert_eq!(diff, dec!(-0.12) - dec!(0.16)); // vs general
    }

    #[test]
    fn unmapped_uses_general() {
        let cats = vec![("Nada a ver".into(), dec!(1000))];
        let (personal, diff) = compute_personal_inflation(&cats, &groups(), dec!(0.16));
        assert_eq!(personal, dec!(0.16));
        assert_eq!(diff, dec!(0));
    }

    #[test]
    fn no_spending_equals_general() {
        let (personal, diff) = compute_personal_inflation(&[], &groups(), dec!(0.16));
        assert_eq!(personal, dec!(0.16));
        assert_eq!(diff, dec!(0));
    }

    #[test]
    fn missing_group_falls_back_to_general() {
        // Educação spent but not in the groups list → general.
        let cats = vec![("Educação".into(), dec!(500))];
        let (personal, _) = compute_personal_inflation(&cats, &groups(), dec!(0.16));
        assert_eq!(personal, dec!(0.16));
    }
}
