# Contract — Senha de fatura por banco (keychain)

**Feature**: 015 · **Camada**: `infrastructure/secrets.rs` + `commands/import.rs`

## Chaves no keychain (serviço `com.financas.app`)

| Banco | USER da credencial | Estado |
|---|---|---|
| BTG | `invoice-password` | **inalterado** — a senha já salva do usuário continua valendo |
| Santander | `invoice-password-santander` | novo |

## API (`secrets.rs`)

```rust
/// Credencial por banco. `bank` é o mesmo identificador do strategy
/// (`InvoiceReader::bank()`): "BTG" → chave legada, outros → "invoice-password-<slug>".
pub fn save_password_for(bank: &str, password: &str) -> Result<(), String>
pub fn get_password_for(bank: &str) -> Option<String>
pub fn clear_password_for(bank: &str) -> Result<(), String>
pub fn has_password_for(bank: &str) -> bool
```

As funções atuais sem sufixo (`get_password()` etc.) viram atalhos BTG ou são migradas nos
call sites — comportamento observável idêntico para o BTG (FR-013).

## Resolução da senha efetiva (`commands/import.rs::import_invoices`)

Por arquivo:

1. Banco = `invoice_reader_for(path)?.bank()`.
2. `password` explícita do caller vence; senão `get_password_for(banco)`.
3. `remember == true` + senha explícita + importação OK ⇒ `save_password_for(banco, senha)`.
   Senha que falhou **nunca** é salva (FR-011).

## Settings

A tela de Configurações que hoje gerencia "senha da fatura" (BTG) passa a existir por
banco (uma entrada Santander ao lado da BTG — limpar/testar), reusando os mesmos comandos
com o parâmetro `bank`. Comandos Tauri afetados: os wrappers de senha existentes ganham o
parâmetro `bank` com default "BTG" (compatibilidade com o front atual até a tela ser
ajustada).
