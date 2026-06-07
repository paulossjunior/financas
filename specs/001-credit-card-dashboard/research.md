# Research: Gestor Financeiro — Dashboard de Faturas BTG

**Branch**: `001-credit-card-dashboard` | **Date**: 2026-06-07

## Critical Finding: BTG XLSX Encryption

**Decision**: BTG invoice files (`.xlsx`) are OLE2-encrypted (CDFV2 Encrypted format). Standard XLSX parsers (calamine, openpyxl) cannot read them without decryption.

**Rationale**: File inspection confirmed magic bytes `D0 CF 11 E0` — Microsoft Compound Document (OLE2) container with encryption layer. `calamine` v0.x does not support encrypted OLE2 files.

**MVP Strategy**: User manually removes password protection using Excel, Numbers, or LibreOffice before placing files in `faturas/`. This is the simplest correct solution for a personal tool.

**Future Enhancement (Post-MVP)**: In-app decryption via `msoffcrypto-tool` Python sidecar called via Tauri sidecar API, accepting CPF or BTG-specific password.

---

## Technology Stack

### Decision: Tauri v2 + Vue 3 + Rust

**Decision**: Use Tauri v2 as the application shell with Rust backend and Vue 3 frontend.

**Rationale**:
- Tauri v2 provides secure desktop app with system file access and no Electron overhead.
- Rust backend enforces data integrity (strong types, no null, explicit error handling).
- Vue 3 + Composition API enables reactive dashboard with minimal boilerplate.
- Entire stack runs locally — no server required, aligns with Local-First principle.

**Alternatives considered**:
- Electron + Node.js: rejected — heavier, higher memory, node's numeric types unsafe for money.
- Python/tkinter local app: rejected — poor developer experience for a dashboard UI.
- Pure Vue SPA with file API: rejected — browser file access sandboxing limits XLSX reading.

---

## XLSX Parsing (Rust)

**Decision**: `calamine` crate for XLSX parsing after user decrypts the file.

**Rationale**: `calamine` is the most mature, well-maintained pure-Rust XLSX/XLS reader. Zero unsafe code, no C FFI, excellent serde integration.

**Version**: calamine 0.25+

**Alternatives considered**:
- `umya-spreadsheet`: full read/write but heavier; write capability not needed.
- Native C library via FFI (libxlsxwriter): brings C unsafety into the domain layer boundary.

---

## Monetary Arithmetic

**Decision**: `rust_decimal` crate (Decimal type) for all monetary values.

**Rationale**: Floating-point arithmetic produces rounding errors unacceptable in financial data (e.g., 0.1 + 0.2 ≠ 0.3 in IEEE 754). `rust_decimal` provides exact base-10 decimal arithmetic.

**Constitution alignment**: Principle IV (Data Integrity) mandates exact decimal arithmetic.

**Alternatives considered**:
- `f64`: rejected — precision errors in sums and percentages.
- Integer cents: viable but less ergonomic; `rust_decimal` is idiomatic and well-tested.

---

## Chart Library (Vue 3)

**Decision**: Apache ECharts via `vue-echarts` wrapper.

**Rationale**: ECharts provides pie charts, bar charts, and line/area charts needed for category breakdown and monthly trend with minimal configuration. `vue-echarts` provides first-class Vue 3 Composition API integration.

**Charts needed**:
- Donut/pie chart: expense breakdown by category
- Horizontal bar chart: category ranking by total
- Line/area chart: monthly trend per category (when multiple invoices present)

**Alternatives considered**:
- Chart.js via vue-chartjs: simpler API but less chart variety; animation and tooltip customization more limited.
- D3.js: maximum flexibility but high implementation cost for a personal tool.

---

## State Management (Vue 3)

**Decision**: Pinia for frontend state management.

**Rationale**: Pinia is the official Vue 3 state management solution, fully typed with TypeScript, devtools-compatible. For this app, one store (`invoice.store.ts`) holds parsed dashboard data.

**Alternatives considered**:
- Vuex 4: legacy, verbose, not recommended for new Vue 3 projects.
- Local `ref`/`reactive`: insufficient for cross-component dashboard data sharing.

---

## Testing Stack

### Rust (backend)
- `cargo test` for unit tests — domain layer logic (categorization, aggregation, decimal math).
- Integration tests in `src-tauri/tests/` — XLSX parsing with real fixture files.
- TDD cycle: write failing test → implement → refactor.

### Vue (frontend)
- **Vitest**: unit and component tests. Co-located with components (`*.test.ts`).
- **Vue Test Utils**: component mounting and interaction in Vitest.
- **Playwright**: E2E tests via `@tauri-apps/cli` test runner against the running Tauri app.

**Constitution alignment**: Principle I (TDD) requires tests written before implementation and coverage ≥ 90% on core logic.

---

## BTG XLSX Column Structure (CONFIRMED — actual file analyzed)

Analysis of decrypted file `2026-06-05_Fatura_Paulo Sérgio Dos Santos Júnior_1302425_BTG-2.xlsx` confirmed the actual format. Sheet name: `Titular`, 203 rows × 8 columns.

### Multi-Section Layout

The BTG XLSX is NOT a simple header + rows table. It has distinct sections:

**Rows 1–17**: Invoice metadata (title, period, due date, payment summary totals).

**Rows 18–24**: Payments section — header at row 20 (`Data | Descrição | Valor`), 3 payment rows.

**Rows 25–32**: Charges/fees section — header at row 27 (`Data | Descrição | Valor`), 4 fee rows (Multa por atraso, IOF, Mora, etc).

**Rows 33–46**: Discount/reversal section — header at row 35 (`Data | Descrição | Valor | Código de autorização | Final Cartão`), negative-amount reversals.

**Rows 47–203**: Main transactions — header at row 47 (`Data | Descrição | Valor | Tipo de compra | Código de autorização | Final Cartão`), 156+ purchase rows.

### Transaction Section Headers (confirmed)

| Column | Section 1 (reversals) | Section 2 (purchases) |
|--------|----------------------|----------------------|
| col 1 | Data | Data |
| col 2 | Descrição | Descrição |
| col 3 | *(empty)* | *(empty)* |
| col 4 | Valor | Valor |
| col 5 | Código de autorização | Tipo de compra |
| col 6 | Final Cartão | Código de autorização |
| col 7 | *(absent)* | Final Cartão |

**Detection criterion**: A row is a transaction section header if it contains columns named `Data`, `Descrição`, AND `Código de autorização`. Payments/fees sections lack the auth code column.

### Installment Encoding

Installments are encoded in the **description text**, not a separate column:
- Example: `"Porto Seguro Seguros (6/10)"` → installment 6 of 10
- Example: `"Leroy Merlin (5/6)"` → installment 5 of 6
- Pattern: `\(N/M\)` suffix in description string

### Date Encoding

Dates are Excel serial numbers (`ExcelDateTime`), not strings. `calamine 0.26` with `dates` feature returns `Data::DateTime(ExcelDateTime)`. Converted via `ExcelDateTime::as_datetime()` → ISO `YYYY-MM-DD` string.

### Tipo de compra Values

- `"Compra à vista"` — single purchase
- `"Parcela sem juros"` — installment, no interest
- `"Compra internacional"` — foreign currency purchase

---

## Category Inference Rules

When `Categoria` column is empty or missing, the system infers category from description using keyword matching:

| Keywords | Category |
|----------|----------|
| IFOOD, UBER EATS, RAPPI, MCDONALDS, restaurante | Alimentação |
| UBER, 99, CABIFY, POSTO, COMBUSTIVEL, PEDÁGIO | Transporte |
| FARMÁCIA, DROGARIA, CLINICA, HOSPITAL, LABORATORIO | Saúde |
| NETFLIX, SPOTIFY, STEAM, CINEMA, INGRESSO | Lazer & Entretenimento |
| AMAZON, SHOPEE, MERCADOLIVRE, AMERICANAS | Compras Online |
| ESCOLA, FACULDADE, CURSO, LIVRARIA, UDEMY | Educação |
| HOTEL, AIRBNB, LATAM, GOLFINHO, DECOLAR | Viagem |
| (no match) | Outros |

Inference rules are stored in a configuration file (`categorias.json`) in the app data directory so the user can customize them over time.

---

## Tauri v2 File System Access

**Decision**: Use `tauri-plugin-fs` for reading files from `faturas/` directory.

**Rationale**: Tauri v2 uses a capability-based security model; `tauri-plugin-fs` provides scoped file system access. The app will request read access to a user-selected directory (or the default `faturas/` relative to app data).

**Implementation note**: Use Tauri's `dialog::open` to let user select the `faturas/` directory on first launch, then persist the path in app config. This avoids hard-coding paths and works cross-platform.
