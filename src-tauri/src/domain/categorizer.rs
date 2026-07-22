use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRule {
    pub keywords: Vec<String>,
    pub category: String,
    pub priority: u32,
}

pub struct Categorizer {
    rules: Vec<CategoryRule>,
}

impl Categorizer {
    pub fn new(mut rules: Vec<CategoryRule>) -> Self {
        rules.sort_by_key(|r| r.priority);
        Self { rules }
    }

    pub fn with_defaults() -> Self {
        Self::new(default_rules())
    }

    pub fn categorize(&self, description: &str) -> String {
        let desc_norm = normalize(description);
        for rule in &self.rules {
            for keyword in &rule.keywords {
                let kw = normalize(keyword);
                if kw.is_empty() {
                    continue;
                }
                if desc_norm.contains(&kw) {
                    return rule.category.clone();
                }
            }
        }
        "Outros".to_string()
    }
}

/// Fold a Portuguese accented character to its ASCII base (á→A, ç→C…).
fn fold(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' | 'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'ç' | 'Ç' => 'C',
        'ñ' | 'Ñ' => 'N',
        other => other,
    }
}

/// Normalize a description or keyword for substring matching.
///
/// Card statements insert auth markers (`*`, `#`) right after the merchant name,
/// e.g. `JIM.COM* 3REX CENTRO`. Normalizing both sides — uppercasing, **folding
/// accents** (so FARMÁCIA matches FARMACIA), turning `*`/`#` into spaces, and
/// collapsing whitespace — makes keyword rules match every variant of a merchant.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true; // start true so leading whitespace is trimmed
    for ch in s.chars() {
        let c = if ch == '*' || ch == '#' { ' ' } else { fold(ch) };
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.extend(c.to_uppercase());
            prev_space = false;
        }
    }
    if out.ends_with(' ') {
        out.pop();
    }
    out
}

fn default_rules() -> Vec<CategoryRule> {
    vec![
        CategoryRule {
            keywords: vec![
                "IFOOD".into(), "UBER EATS".into(), "RAPPI".into(),
                "MCDONALDS".into(), "RESTAURANTE".into(), "LANCHONETE".into(),
                "PADARIA".into(), "PIZZARIA".into(), "DELIVERY".into(),
                "BURGER".into(), "SUSHI".into(),
            ],
            category: "Alimentação".into(),
            priority: 10,
        },
        CategoryRule {
            keywords: vec![
                "UBER".into(), "99".into(), "CABIFY".into(),
                "POSTO".into(), "COMBUSTIVEL".into(), "GASOLINA".into(),
                "PEDAGIO".into(), "ESTACIONAMENTO".into(), "METRÔ".into(),
                "METRO".into(), "ONIBUS".into(), "BUS".into(),
            ],
            category: "Transporte".into(),
            priority: 20,
        },
        CategoryRule {
            keywords: vec![
                "FARMACIA".into(), "DROGARIA".into(), "CLINICA".into(),
                "HOSPITAL".into(), "LABORATORIO".into(), "MEDICO".into(),
                "DENTISTA".into(), "UNIMED".into(), "PLANO DE SAUDE".into(),
                "SULAMERICA".into(), "AMIL".into(),
            ],
            category: "Saúde".into(),
            priority: 30,
        },
        CategoryRule {
            keywords: vec![
                "NETFLIX".into(), "SPOTIFY".into(), "STEAM".into(),
                "CINEMA".into(), "INGRESSO".into(), "DISNEY".into(),
                "HBO".into(), "YOUTUBE".into(), "PRIME".into(),
                "APPLE TV".into(), "GLOBOPLAY".into(),
            ],
            category: "Lazer & Entretenimento".into(),
            priority: 40,
        },
        CategoryRule {
            keywords: vec![
                "AMAZON".into(), "SHOPEE".into(), "MERCADOLIVRE".into(),
                "AMERICANAS".into(), "SUBMARINO".into(), "CASAS BAHIA".into(),
                "MAGALU".into(), "MAGAZINE".into(), "ALIEXPRESS".into(),
            ],
            category: "Compras Online".into(),
            priority: 50,
        },
        CategoryRule {
            keywords: vec![
                "ESCOLA".into(), "FACULDADE".into(), "CURSO".into(),
                "LIVRARIA".into(), "UDEMY".into(), "ALURA".into(),
                "COURSERA".into(), "DUOLINGO".into(),
            ],
            category: "Educação".into(),
            priority: 60,
        },
        CategoryRule {
            keywords: vec![
                "HOTEL".into(), "AIRBNB".into(), "LATAM".into(),
                "GOL".into(), "AZUL".into(), "DECOLAR".into(),
                "BOOKING".into(), "HOSTEL".into(),
            ],
            category: "Viagem".into(),
            priority: 70,
        },
        CategoryRule {
            keywords: vec![
                "INTERNET".into(), "TELEFONE".into(), "CLARO".into(),
                "VIVO".into(), "TIM".into(), "OI ".into(),
                "ENERGIA".into(), "AGUA".into(), "GAS".into(),
                "CONDOMINIO".into(), "ALUGUEL".into(),
            ],
            category: "Moradia & Serviços".into(),
            priority: 80,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_match_alimentacao() {
        let c = Categorizer::with_defaults();
        assert_eq!(c.categorize("IFOOD*RESTAURANTE X"), "Alimentação");
        assert_eq!(c.categorize("MCDONALDS CENTRO"), "Alimentação");
    }

    #[test]
    fn test_keyword_match_transporte() {
        let c = Categorizer::with_defaults();
        assert_eq!(c.categorize("UBER TRIP SAO PAULO"), "Transporte");
    }

    #[test]
    fn test_no_match_returns_outros() {
        let c = Categorizer::with_defaults();
        assert_eq!(c.categorize("XPTO STORE UNKN"), "Outros");
    }

    #[test]
    fn test_case_insensitive_match() {
        let c = Categorizer::with_defaults();
        assert_eq!(c.categorize("ifood delivery"), "Alimentação");
    }

    #[test]
    fn test_custom_rule_takes_priority() {
        let rules = vec![
            CategoryRule {
                keywords: vec!["XPTO".into()],
                category: "Custom".into(),
                priority: 0,
            },
        ];
        let c = Categorizer::new(rules);
        assert_eq!(c.categorize("XPTO STORE"), "Custom");
    }

    #[test]
    fn test_keyword_across_auth_marker_matches_all_variants() {
        // Tag derived without the `*` must still match every raw variant that carries it.
        let rules = vec![CategoryRule {
            keywords: vec!["JIM.COM 3REX CENTRO".into()],
            category: "Saúde".into(),
            priority: 5,
        }];
        let c = Categorizer::new(rules);
        assert_eq!(c.categorize("Jim.com* 3rex Centro"), "Saúde");
        assert_eq!(c.categorize("JIM.COM *3REX CENTRO SP"), "Saúde");
        // A different Jim.com merchant is correctly NOT captured.
        assert_eq!(c.categorize("Jim.com* Almeia Angel"), "Outros");
    }

    #[test]
    fn test_short_gateway_keyword_catches_all() {
        // Shortening the tag to the gateway name captures every sub-merchant.
        let rules = vec![CategoryRule {
            keywords: vec!["JIM.COM".into()],
            category: "Saúde".into(),
            priority: 5,
        }];
        let c = Categorizer::new(rules);
        assert_eq!(c.categorize("Jim.com* 3rex Centro"), "Saúde");
        assert_eq!(c.categorize("Jim.com* Almeia Angel"), "Saúde");
    }

    #[test]
    fn accent_insensitive_and_root_match() {
        let rules = vec![CategoryRule {
            keywords: vec!["FARMACIA".into(), "DROGA".into(), "ACOUGUE".into()],
            category: "Saúde".into(),
            priority: 5,
        }];
        let c = Categorizer::new(rules);
        assert_eq!(c.categorize("FARMÁCIA INDIANA"), "Saúde"); // accent folded
        assert_eq!(c.categorize("Drogasil Filial"), "Saúde"); // root DROGA
        assert_eq!(c.categorize("AÇOUGUE SANTA LUCIA"), "Saúde"); // ç folded
    }

    #[test]
    fn test_empty_keyword_does_not_match_everything() {
        let rules = vec![CategoryRule {
            keywords: vec!["*".into(), "".into()],
            category: "Bug".into(),
            priority: 0,
        }];
        let c = Categorizer::new(rules);
        assert_eq!(c.categorize("QUALQUER COISA"), "Outros");
    }
}
