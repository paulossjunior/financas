//! Rigorous personal-inflation calculation (pure domain, deterministic).
//!
//! Given per-category {spend, category inflation} plus {income, official inflation,
//! optional behavioral coefficient}, compute personal inflation as the spend-weighted
//! average of category inflations, plus the derived figures (contributions, official
//! comparison in **percentage points**, updated basket cost, income needed to keep
//! purchasing power, purchasing-power loss, and an optional behavioral simulation).
//!
//! Money is [`Decimal`]; rates are `f64` (period conversion needs fractional powers).
//! This module does NO I/O — the application layer supplies the inputs (spends from the
//! dashboard, per-category inflation from the IPCA groups, official from the headline).

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Default behavioral coefficient: ~1.4% extra real consumption per +1 p.p. of personal
/// inflation (from the referenced econometric study). Configurable; simulation only.
pub const DEFAULT_BEHAVIORAL_COEFFICIENT: f64 = 1.4;

pub const METHODOLOGY_NOTE: &str = "A inflação pessoal é uma estimativa baseada na distribuição dos seus gastos e nas taxas de inflação atribuídas a cada categoria; depende da qualidade, da periodicidade e da compatibilidade dos dados. A estimativa de impacto sobre o consumo usa um coeficiente econométrico obtido em outro contexto e não representa previsão individual, perda financeira direta nem recomendação.";

/// Which weights to use for the personal index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightMode {
    /// Weights from the current period's spends (default).
    #[default]
    Current,
    /// Weights from a fixed base-period basket (`base_gasto`).
    Base,
}

/// One category's input.
#[derive(Debug, Clone)]
pub struct CategoryInput {
    pub category: String,
    /// Current-period spend (money).
    pub gasto: Decimal,
    /// Base-period spend (money), required only for `WeightMode::Base`.
    pub base_gasto: Option<Decimal>,
    /// Category inflation for the period (rate; may be 0 or negative).
    pub inflacao: f64,
    /// Set when this category's inflation was borrowed from an aggregate group.
    pub provenance: Option<String>,
}

impl CategoryInput {
    pub fn new(category: impl Into<String>, gasto: Decimal, inflacao: f64) -> Self {
        Self { category: category.into(), gasto, base_gasto: None, inflacao, provenance: None }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contribution {
    pub category: String,
    pub weight: f64,
    pub inflacao: f64,
    pub contribuicao: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonalInflationResult {
    #[serde(with = "rust_decimal::serde::str")]
    pub gasto_total: Decimal,
    pub inflacao_pessoal: f64,
    pub inflacao_pessoal_pct: f64,
    pub inflacao_oficial: f64,
    /// Personal − official, in **percentage points**.
    pub diferenca_pp: f64,
    #[serde(with = "rust_decimal::serde::str")]
    pub custo_atualizado: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub aumento_cesta: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub renda_corrigida: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub aumento_renda: Decimal,
    /// Conservative variant: income raised only by the extra basket cost.
    #[serde(with = "rust_decimal::serde::str")]
    pub renda_corrigida_consumo: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub perda_poder_compra: Decimal,
    pub impacto_comportamental: Option<f64>,
    pub impacto_comportamental_pct: Option<f64>,
    #[serde(with = "rust_decimal::serde::str_option")]
    pub consumo_adicional: Option<Decimal>,
    pub contribuicoes: Vec<Contribution>,
    pub proveniencias: Vec<String>,
    pub aviso: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersonalInflationError {
    EmptyCategories,
    NonPositiveTotal,
    NegativeGasto(String),
    DuplicateCategory(String),
    MissingBaseGasto(String),
}

impl std::fmt::Display for PersonalInflationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersonalInflationError::EmptyCategories => write!(f, "nenhuma categoria informada"),
            PersonalInflationError::NonPositiveTotal => write!(f, "gasto total deve ser maior que zero"),
            PersonalInflationError::NegativeGasto(c) => write!(f, "gasto negativo em '{c}'"),
            PersonalInflationError::DuplicateCategory(c) => write!(f, "categoria duplicada: '{c}'"),
            PersonalInflationError::MissingBaseGasto(c) => write!(f, "gasto do período-base ausente em '{c}'"),
        }
    }
}

fn to_f64(d: Decimal) -> f64 {
    d.to_f64().unwrap_or(0.0)
}
fn dec(x: f64) -> Decimal {
    Decimal::from_f64_retain(x).unwrap_or_default()
}

/// Compute personal inflation and all derived figures.
pub fn compute(
    categories: &[CategoryInput],
    inflacao_oficial: f64,
    renda: Decimal,
    coeficiente: Option<f64>,
    weight_mode: WeightMode,
) -> Result<PersonalInflationResult, PersonalInflationError> {
    if categories.is_empty() {
        return Err(PersonalInflationError::EmptyCategories);
    }
    let mut seen = std::collections::HashSet::new();
    for c in categories {
        if c.gasto < Decimal::ZERO {
            return Err(PersonalInflationError::NegativeGasto(c.category.clone()));
        }
        if !seen.insert(c.category.clone()) {
            return Err(PersonalInflationError::DuplicateCategory(c.category.clone()));
        }
    }

    let gasto_total: Decimal = categories.iter().map(|c| c.gasto).sum();
    if gasto_total <= Decimal::ZERO {
        return Err(PersonalInflationError::NonPositiveTotal);
    }

    // Weight denominator depends on the mode.
    let weight_denom: Decimal = match weight_mode {
        WeightMode::Current => gasto_total,
        WeightMode::Base => {
            let mut sum = Decimal::ZERO;
            for c in categories {
                match c.base_gasto {
                    Some(b) => sum += b,
                    None => return Err(PersonalInflationError::MissingBaseGasto(c.category.clone())),
                }
            }
            if sum <= Decimal::ZERO {
                return Err(PersonalInflationError::NonPositiveTotal);
            }
            sum
        }
    };
    let denom_f = to_f64(weight_denom);

    let mut contribuicoes = Vec::with_capacity(categories.len());
    let mut proveniencias = Vec::new();
    let mut inflacao_pessoal = 0.0_f64;
    for c in categories {
        let numer = match weight_mode {
            WeightMode::Current => to_f64(c.gasto),
            WeightMode::Base => to_f64(c.base_gasto.unwrap_or(Decimal::ZERO)),
        };
        let weight = if denom_f != 0.0 { numer / denom_f } else { 0.0 };
        let contribuicao = weight * c.inflacao;
        inflacao_pessoal += contribuicao;
        contribuicoes.push(Contribution {
            category: c.category.clone(),
            weight,
            inflacao: c.inflacao,
            contribuicao,
        });
        if let Some(p) = &c.provenance {
            proveniencias.push(p.clone());
        }
    }
    // Largest contribution first.
    contribuicoes.sort_by(|a, b| b.contribuicao.partial_cmp(&a.contribuicao).unwrap_or(std::cmp::Ordering::Equal));

    let factor = dec(1.0 + inflacao_pessoal);
    let custo_atualizado = (gasto_total * factor).round_dp(2);
    let aumento_cesta = (custo_atualizado - gasto_total).round_dp(2);
    let renda_corrigida = (renda * factor).round_dp(2);
    let aumento_renda = (renda_corrigida - renda).round_dp(2);
    let perda_poder_compra = (gasto_total * dec(inflacao_pessoal)).round_dp(2);
    // Conservative: raise income only by the extra basket cost.
    let renda_corrigida_consumo = (renda + perda_poder_compra).round_dp(2);

    let (impacto_comportamental, impacto_comportamental_pct, consumo_adicional) = match coeficiente {
        Some(coef) => {
            let pp = inflacao_pessoal * 100.0;
            let impacto_pct = pp * coef; // e.g. 7.7 * 1.4 = 10.78 (%)
            let impacto_dec = impacto_pct / 100.0; // 0.1078
            let consumo = (gasto_total * dec(impacto_dec)).round_dp(2);
            (Some(impacto_dec), Some(impacto_pct), Some(consumo))
        }
        None => (None, None, None),
    };

    Ok(PersonalInflationResult {
        gasto_total,
        inflacao_pessoal,
        inflacao_pessoal_pct: inflacao_pessoal * 100.0,
        inflacao_oficial,
        diferenca_pp: (inflacao_pessoal - inflacao_oficial) * 100.0,
        custo_atualizado,
        aumento_cesta,
        renda_corrigida,
        aumento_renda,
        renda_corrigida_consumo,
        perda_poder_compra,
        impacto_comportamental,
        impacto_comportamental_pct,
        consumo_adicional,
        contribuicoes,
        proveniencias,
        aviso: METHODOLOGY_NOTE.to_string(),
    })
}

// ── Period conversion & accumulation (composite interest; never divide annual by 12) ──

/// Annual rate → monthly rate (compound): (1+a)^(1/12) − 1.
pub fn annual_to_monthly(annual: f64) -> f64 {
    (1.0 + annual).powf(1.0 / 12.0) - 1.0
}
/// Monthly rate → annual rate (compound): (1+m)^12 − 1.
pub fn monthly_to_annual(monthly: f64) -> f64 {
    (1.0 + monthly).powi(12) - 1.0
}
/// Quarterly rate → monthly rate (compound).
pub fn quarterly_to_monthly(quarterly: f64) -> f64 {
    (1.0 + quarterly).powf(1.0 / 3.0) - 1.0
}
/// Accumulate a series of period rates by product: ∏(1+π) − 1 (never sum).
pub fn accumulate(rates: &[f64]) -> f64 {
    rates.iter().fold(1.0, |acc, r| acc * (1.0 + r)) - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec as d;

    const EPS: f64 = 1e-9;

    fn ref_categories() -> Vec<CategoryInput> {
        vec![
            CategoryInput::new("Alimentação", d!(2000), 0.10),
            CategoryInput::new("Transporte", d!(1500), 0.08),
            CategoryInput::new("Habitação", d!(1000), 0.05),
            CategoryInput::new("Outros", d!(500), 0.03),
        ]
    }

    #[test]
    fn reference_example_matches_spec() {
        let r = compute(&ref_categories(), 0.06, d!(7000), Some(1.4), WeightMode::Current).unwrap();
        assert_eq!(r.gasto_total, d!(5000));
        assert!((r.inflacao_pessoal - 0.077).abs() < EPS);
        assert!((r.inflacao_pessoal_pct - 7.7).abs() < 1e-7);
        assert!((r.diferenca_pp - 1.7).abs() < 1e-7);
        assert_eq!(r.custo_atualizado, d!(5385.00));
        assert_eq!(r.aumento_cesta, d!(385.00));
        assert_eq!(r.renda_corrigida, d!(7539.00));
        assert_eq!(r.aumento_renda, d!(539.00));
        assert_eq!(r.perda_poder_compra, d!(385.00));
        assert!((r.impacto_comportamental_pct.unwrap() - 10.78).abs() < 1e-6);
        assert!((r.impacto_comportamental.unwrap() - 0.1078).abs() < EPS);
        assert_eq!(r.consumo_adicional.unwrap(), d!(539.00));
    }

    #[test]
    fn contributions_sum_to_personal_and_are_sorted() {
        let r = compute(&ref_categories(), 0.06, d!(7000), None, WeightMode::Current).unwrap();
        let sum: f64 = r.contribuicoes.iter().map(|c| c.contribuicao).sum();
        assert!((sum - r.inflacao_pessoal).abs() < EPS);
        // sorted desc
        assert_eq!(r.contribuicoes[0].category, "Alimentação");
        assert!(r.contribuicoes.windows(2).all(|w| w[0].contribuicao >= w[1].contribuicao));
        // weights sum to 1
        let wsum: f64 = r.contribuicoes.iter().map(|c| c.weight).sum();
        assert!((wsum - 1.0).abs() < EPS);
    }

    #[test]
    fn behavioral_omitted_when_no_coefficient() {
        let r = compute(&ref_categories(), 0.06, d!(7000), None, WeightMode::Current).unwrap();
        assert!(r.impacto_comportamental.is_none());
        assert!(r.consumo_adicional.is_none());
    }

    #[test]
    fn single_category() {
        let cats = vec![CategoryInput::new("Tudo", d!(1000), 0.05)];
        let r = compute(&cats, 0.04, d!(3000), None, WeightMode::Current).unwrap();
        assert!((r.inflacao_pessoal - 0.05).abs() < EPS);
        assert!((r.contribuicoes[0].weight - 1.0).abs() < EPS);
    }

    #[test]
    fn zero_inflation() {
        let cats = vec![
            CategoryInput::new("A", d!(100), 0.0),
            CategoryInput::new("B", d!(100), 0.0),
        ];
        let r = compute(&cats, 0.0, d!(1000), None, WeightMode::Current).unwrap();
        assert!(r.inflacao_pessoal.abs() < EPS);
        assert_eq!(r.custo_atualizado, d!(200.00));
        assert_eq!(r.aumento_cesta, d!(0.00));
    }

    #[test]
    fn deflation_reduces_personal() {
        let cats = vec![
            CategoryInput::new("A", d!(500), -0.02),
            CategoryInput::new("B", d!(500), 0.02),
        ];
        let r = compute(&cats, 0.0, d!(1000), None, WeightMode::Current).unwrap();
        assert!(r.inflacao_pessoal.abs() < EPS); // -0.01 + 0.01
    }

    #[test]
    fn zero_total_is_error() {
        let cats = vec![CategoryInput::new("A", d!(0), 0.05)];
        assert_eq!(compute(&cats, 0.0, d!(0), None, WeightMode::Current), Err(PersonalInflationError::NonPositiveTotal));
    }

    #[test]
    fn negative_and_duplicate_are_errors() {
        let neg = vec![CategoryInput::new("A", d!(-1), 0.0)];
        assert_eq!(compute(&neg, 0.0, d!(0), None, WeightMode::Current), Err(PersonalInflationError::NegativeGasto("A".into())));
        let dup = vec![CategoryInput::new("A", d!(1), 0.0), CategoryInput::new("A", d!(1), 0.0)];
        assert_eq!(compute(&dup, 0.0, d!(0), None, WeightMode::Current), Err(PersonalInflationError::DuplicateCategory("A".into())));
    }

    #[test]
    fn empty_is_error() {
        assert_eq!(compute(&[], 0.0, d!(0), None, WeightMode::Current), Err(PersonalInflationError::EmptyCategories));
    }

    #[test]
    fn base_weights_differ_from_current() {
        // Current spends shifted toward the high-inflation category, base is balanced.
        let cats = vec![
            CategoryInput { category: "Alta".into(), gasto: d!(900), base_gasto: Some(d!(500)), inflacao: 0.10, provenance: None },
            CategoryInput { category: "Baixa".into(), gasto: d!(100), base_gasto: Some(d!(500)), inflacao: 0.00, provenance: None },
        ];
        let cur = compute(&cats, 0.0, d!(1000), None, WeightMode::Current).unwrap();
        let base = compute(&cats, 0.0, d!(1000), None, WeightMode::Base).unwrap();
        // current: 0.9*0.10 = 0.09 ; base: 0.5*0.10 = 0.05
        assert!((cur.inflacao_pessoal - 0.09).abs() < EPS);
        assert!((base.inflacao_pessoal - 0.05).abs() < EPS);
    }

    #[test]
    fn missing_base_gasto_is_error() {
        let cats = vec![CategoryInput::new("A", d!(100), 0.05)]; // base_gasto None
        assert_eq!(compute(&cats, 0.0, d!(0), None, WeightMode::Base), Err(PersonalInflationError::MissingBaseGasto("A".into())));
    }

    #[test]
    fn provenance_is_reported() {
        let mut cats = ref_categories();
        cats[3].provenance = Some("usou Transportes para Combustível".into());
        let r = compute(&cats, 0.06, d!(7000), None, WeightMode::Current).unwrap();
        assert_eq!(r.proveniencias, vec!["usou Transportes para Combustível".to_string()]);
    }

    #[test]
    fn period_conversion_compound_not_divided() {
        let m = annual_to_monthly(0.06);
        assert!((m - 0.0048675).abs() < 1e-6); // ≈ 0,4868%, not 0.5%
        assert!(m < 0.06 / 12.0); // strictly less than the naive division
        let a = monthly_to_annual(m);
        assert!((a - 0.06).abs() < 1e-9); // round-trips
    }

    #[test]
    fn accumulate_uses_product_not_sum() {
        let acc = accumulate(&[0.01, 0.02, 0.005]);
        let expected = 1.01 * 1.02 * 1.005 - 1.0;
        assert!((acc - expected).abs() < EPS);
        assert!(acc > 0.01 + 0.02 + 0.005 - 1e-9 - 0.0 && (acc - (0.01 + 0.02 + 0.005)).abs() > 1e-6); // differs from naive sum
    }

    #[test]
    fn methodology_note_present() {
        let r = compute(&ref_categories(), 0.06, d!(7000), None, WeightMode::Current).unwrap();
        assert!(r.aviso.contains("estimativa"));
        assert!(!r.aviso.is_empty());
    }
}
