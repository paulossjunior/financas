# Contract — Leitor de fatura Santander

**Feature**: 015 · **Camadas**: `domain/santander_invoice.rs` (puro) +
`infrastructure/santander_invoice.rs` (I/O + strategy)

## Domínio (puro, testável com fixture de texto)

```rust
/// true quando o texto extraído é uma fatura Santander (marcadores estruturais:
/// "Detalhamento da Fatura" + ("Resumo da Fatura" | "BANCO SANTANDER")).
/// Contracheque SouGov e extrato Banestes retornam false.
pub fn is_santander_invoice(text: &str) -> bool
```

```rust
impl FaturaSantander {
    /// Texto extraído → struct tipada. Estrutura apenas; sem juízo de integridade.
    /// Err pt-BR: "Este PDF não é uma fatura do Santander." | "Não reconheci o formato…"
    pub fn parse(text: &str) -> Result<Self, String>;

    /// Checagens de integridade contra o Resumo da Fatura (R7). Relatório puro.
    pub fn conferir(&self) -> Conferencia;

    /// Mês de referência: Fatura_MMYYYY do nome do arquivo; fallback vencimento (R10).
    pub fn reference_month(&self, filename: &str) -> YearMonth;

    /// Compras+IOFs+créditos → transações categorizadas, na ordem do PDF (row_index
    /// sequencial → ids determinísticos). Pagamentos de fatura NÃO geram transação.
    pub fn into_transactions(self, invoice_id: Uuid, categorizer: &Categorizer)
        -> (Vec<Transaction>, Vec<ParseWarning>);
}
```

```rust
impl Conferencia {
    /// Política estrita (mesma da 014): Divergiu OU SemDados ⇒ Err pt-BR com a
    /// diferença em R$ ("A leitura da fatura não fechou … (diferença de R$ X). Nada
    /// foi importado.") ou com o que faltou. Fechou+Fechou ⇒ Ok.
    pub fn exigir(&self) -> Result<(), String>;
}
```

## Infraestrutura (casca de I/O + strategy)

```rust
/// PDF → texto. Sem senha tenta aberto; se o PDF for cifrado exige senha.
/// Erros mapeados: cifrado sem senha → InvoiceReadError::Encrypted;
/// senha errada → InvoiceReadError::WrongPassword; demais → Io/InvalidFormat.
pub fn extract_text(path: &str, password: Option<&str>) -> Result<String, InvoiceReadError>

/// true quando o PDF é cifrado (candidato Santander na pasta automática — R8 nível 2).
pub fn is_encrypted_pdf(path: &str) -> bool

pub struct SantanderInvoiceReader;
impl InvoiceReader for SantanderInvoiceReader {
    fn bank(&self) -> &'static str { "Santander" }
    fn extensions(&self) -> &'static [&'static str] { &["pdf"] }
    fn read(&self, path, password, invoice_id, categorizer)
        -> Result<(Vec<Transaction>, Vec<ParseWarning>), InvoiceReadError>;
        // extract_text → is_santander_invoice (senão InvalidFormat) → parse →
        // conferir().exigir() (senão InvalidFormat com a mensagem pt-BR) → into_transactions
}
```

Registro: `INVOICE_READERS = [&BtgInvoiceReader, &SantanderInvoiceReader]` — despacho por
extensão continua unívoco (`xlsx` → BTG, `pdf` → Santander).

## Mês de referência no `import_invoice`

`import_invoice` hoje infere o mês pelo padrão BTG `YYYY-MM-…`. O contrato ganha: quando o
nome não casa o padrão BTG, o leitor informa o mês (`Fatura_MMYYYY` / vencimento). A
implementação escolhe o ponto de menor atrito (ex.: o reader devolve também o
`reference_month`, ou o `infer_month_from_filename` aprende o segundo padrão) — decisão
registrada em tasks.md; o comportamento BTG não muda (FR-013).

## Erros (user-facing, pt-BR — todos com letra acentuada, regra do mapError da 014)

| Situação | Mensagem/código |
|---|---|
| PDF cifrado sem senha | `ENCRYPTED_FILE` (front já pede senha) |
| Senha errada | `WRONG_PASSWORD` (front pede de novo; não salva) |
| Não é fatura Santander | "Este PDF não é uma fatura do Santander." |
| Conferência divergiu | "A leitura da fatura não fechou com o resumo impresso (diferença de R$ X). Nada foi importado." |
| Resumo ausente | "Não encontrei o resumo da fatura para conferir a leitura. Nada foi importado." |
| Fatura sem transações | importa com 0 transações (não é erro) |

## Pasta automática (`import_folder`, ramo `.pdf`)

Ordem de decisão por arquivo `.pdf`:

1. `StatementReader` (Banestes) `recognizes`? → importa como extrato (fluxo 014).
2. Senão, `is_encrypted_pdf`? → candidato a fatura Santander:
   - senha salva (`invoice-password-santander`) → `SantanderInvoiceReader.read`;
     sucesso → fatura importada; falha → `ignored` com `ERROR: …`/`WRONG_PASSWORD`.
   - sem senha salva → `ignored` com `ENCRYPTED_NO_PASSWORD` (código existente da 013).
3. Senão (PDF aberto que não é extrato — contracheque): silêncio (política da 014).
```
