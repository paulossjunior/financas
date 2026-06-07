use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryRule {
    pub keywords: Vec<String>,
    pub category: String,
    pub priority: u8,
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
        let desc_upper = description.to_uppercase();
        for rule in &self.rules {
            for keyword in &rule.keywords {
                if desc_upper.contains(&keyword.to_uppercase()) {
                    return rule.category.clone();
                }
            }
        }
        "Outros".to_string()
    }
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
}
