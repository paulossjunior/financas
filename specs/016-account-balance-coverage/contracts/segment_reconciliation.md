# Contract — Conferência por segmento (parser Banestes)

**Feature**: 016 · **Camada**: `domain/banestes_statement.rs`

## Captura

Durante a varredura existente, cada linha de saldo intermediária
(`Saldo <valor>` — as variantes `JUL/26 Saldo`, `Saldo` puro; **não** `Saldo Anterior` /
`Saldo Conta` / `Saldo Total` / `Saldo Poupança`) fecha um segmento:

```rust
pub struct Segmento {
    /// Dia da coluna do último movimento do trecho (rótulo do aviso).
    pub dia: Option<u32>,
    /// Saldo impresso que fecha o trecho.
    pub saldo_impresso: Decimal,
    /// Σ movimentos do trecho (créditos − débitos).
    pub soma_trecho: Decimal,
}
```

`ExtratoBanestes.segmentos: Vec<Segmento>`; o primeiro trecho abre no `Saldo Anterior`.

## Checagem

`Conferencia` ganha `segmentos: Checagem`:

- `Fechou`: para todo segmento i, `saldo_{i-1} + soma_trecho_i == saldo_impresso_i`
  (saldo_0 = Saldo Anterior).
- `Divergiu { diferenca }`: primeiro segmento que não fecha; a mensagem do `exigir()` cita
  o dia: `"A leitura do extrato não fechou no dia DD (diferença de R$ X). Nada foi
  importado."`.
- `SemDados`: extrato sem nenhum saldo intermediário. **Tolerado** (não vira erro) — a
  conferência total continua obrigatória; documentar no `exigir()` que esta é a única
  checagem cuja ausência não bloqueia (spec US3 cenário 3).

## Propriedade que a suíte deve provar

Fixture `banestes_extrato_autocancela.txt`: cópia da principal com **+100,00 numa linha e
−100,00 em outra** (dias diferentes) — a soma total continua fechando com os saldos e com
entradas/saídas declaradas, e **só** a checagem de segmentos acusa. É o teste que separa
esta rede da anterior.
