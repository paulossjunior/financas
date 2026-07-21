//! IBGE fetch (the app's only network call — opt-in, read-only public data).
//! Pulls the IPCA headline (aggregate 1737) and the per-group monthly variation
//! (aggregate 7060) and builds a cacheable snapshot.

use std::str::FromStr;
use std::time::Duration;

use rust_decimal::Decimal;
use serde_json::Value;

use crate::domain::inflation::{InflationCache, IpcaGroup, IpcaHeadline, IPCA_GROUPS};

const HEADLINE_URL: &str =
    "https://servicodados.ibge.gov.br/api/v3/agregados/1737/periodos/-1/variaveis/63|2265|69?localidades=N1[1]";
const GROUPS_URL: &str =
    "https://servicodados.ibge.gov.br/api/v3/agregados/7060/periodos/-1/variaveis/63?localidades=N1[1]&classificacao=315[all]";

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s.trim()).unwrap_or(Decimal::ZERO)
}

/// Last (period, value) of a variable object's first series. serde_json maps are
/// sorted, so the last key is the most recent period.
fn last_serie(var_obj: &Value) -> Option<(String, String)> {
    let serie = var_obj
        .get("resultados")?
        .get(0)?
        .get("series")?
        .get(0)?
        .get("serie")?
        .as_object()?;
    serie
        .iter()
        .next_back()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
}

/// "202606" -> "2026-06"
fn ref_month(period: &str) -> String {
    if period.len() == 6 {
        format!("{}-{}", &period[0..4], &period[4..6])
    } else {
        period.to_string()
    }
}

/// Strip a leading "N." numbering ("1.Alimentação e bebidas" -> "Alimentação e bebidas").
fn clean_group_name(raw: &str) -> String {
    if let Some((prefix, rest)) = raw.split_once('.') {
        if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
            return rest.trim().to_string();
        }
    }
    raw.trim().to_string()
}

fn parse_headline(json: &Value) -> Result<IpcaHeadline, String> {
    let arr = json.as_array().ok_or("IBGE: resposta headline inesperada")?;
    let mut month = Decimal::ZERO;
    let mut year = Decimal::ZERO;
    let mut twelve = Decimal::ZERO;
    let mut period = String::new();
    for var in arr {
        let id = var.get("id").and_then(|v| v.as_str()).unwrap_or("");
        if let Some((p, val)) = last_serie(var) {
            period = p;
            match id {
                "63" => month = dec(&val),
                "2265" => year = dec(&val),
                "69" => twelve = dec(&val),
                _ => {}
            }
        }
    }
    if period.is_empty() {
        return Err("IBGE: não achei o período do IPCA".into());
    }
    Ok(IpcaHeadline { ref_month: ref_month(&period), month, year, twelve })
}

fn parse_groups(json: &Value) -> Vec<IpcaGroup> {
    let mut out = Vec::new();
    let Some(var) = json.as_array().and_then(|a| a.first()) else {
        return out;
    };
    let Some(resultados) = var.get("resultados").and_then(|v| v.as_array()) else {
        return out;
    };
    for r in resultados {
        // classificacoes[0].categoria = { "<code>": "<name>" }
        let name = r
            .get("classificacoes")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|c| c.get("categoria"))
            .and_then(|v| v.as_object())
            .and_then(|m| m.values().next())
            .and_then(|v| v.as_str())
            .map(clean_group_name)
            .unwrap_or_default();
        if name.is_empty() || !IPCA_GROUPS.contains(&name.as_str()) {
            continue;
        }
        let value = r
            .get("series")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|s| s.get("serie"))
            .and_then(|v| v.as_object())
            .and_then(|m| m.values().next_back())
            .and_then(|v| v.as_str())
            .unwrap_or("0");
        out.push(IpcaGroup { name, month_var: dec(value) });
    }
    out
}

/// Fetch IPCA headline + groups from the IBGE public API. Network only.
pub async fn fetch_inflation() -> Result<InflationCache, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("financas-app")
        .build()
        .map_err(|e| e.to_string())?;

    let get = |url: &'static str| {
        let c = client.clone();
        async move {
            c.get(url)
                .send()
                .await
                .map_err(|e| format!("Falha ao acessar o IBGE: {e}"))?
                .json::<Value>()
                .await
                .map_err(|e| format!("Resposta do IBGE inválida: {e}"))
        }
    };

    let headline_json = get(HEADLINE_URL).await?;
    let groups_json = get(GROUPS_URL).await?;

    let headline = parse_headline(&headline_json)?;
    let groups = parse_groups(&groups_json);
    if groups.is_empty() {
        return Err("IBGE: não consegui ler os grupos do IPCA".into());
    }
    let fetched_at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    Ok(InflationCache { headline, groups, fetched_at })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headline_from_json() {
        let json: Value = serde_json::from_str(
            r#"[
              {"id":"63","resultados":[{"series":[{"serie":{"202605":"0.58","202606":"0.16"}}]}]},
              {"id":"2265","resultados":[{"series":[{"serie":{"202606":"3.36"}}]}]},
              {"id":"69","resultados":[{"series":[{"serie":{"202606":"4.64"}}]}]}
            ]"#,
        )
        .unwrap();
        let h = parse_headline(&json).unwrap();
        assert_eq!(h.ref_month, "2026-06");
        assert_eq!(h.month.to_string(), "0.16");
        assert_eq!(h.year.to_string(), "3.36");
        assert_eq!(h.twelve.to_string(), "4.64");
    }

    #[test]
    fn parses_groups_and_strips_numbering() {
        let json: Value = serde_json::from_str(
            r#"[{"id":"63","resultados":[
              {"classificacoes":[{"categoria":{"7169":"Índice geral"}}],"series":[{"serie":{"202606":"0.16"}}]},
              {"classificacoes":[{"categoria":{"7170":"1.Alimentação e bebidas"}}],"series":[{"serie":{"202606":"-0.24"}}]},
              {"classificacoes":[{"categoria":{"7445":"2.Habitação"}}],"series":[{"serie":{"202606":"0.63"}}]}
            ]}]"#,
        )
        .unwrap();
        let g = parse_groups(&json);
        // "Índice geral" is not one of the 9 groups → excluded.
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].name, "Alimentação e bebidas");
        assert_eq!(g[0].month_var.to_string(), "-0.24");
        assert_eq!(g[1].name, "Habitação");
    }
}
