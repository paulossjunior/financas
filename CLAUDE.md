<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan
at specs/008-bank-statement-import/plan.md
<!-- SPECKIT END -->

## Project overview

Desktop personal-finance app (Tauri v2 + Rust + Vue 3 + SQLite). Imports BTG
card invoices (.xlsx, possibly encrypted) and SouGov.br payslips (PDF); shows a
monthly dashboard, an annual view, and PDF reports. 100% local, no network.

- **README**: [README.md](README.md) — features, install, dev, build, CI/CD.
- **Architecture**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — layers, data/money model, flows, DB, commands.
- **Specs** (spec-kit): `specs/001-credit-card-dashboard`, `002-monthly-invoice-list`,
  `003-custom-categories`, `004-payslips-annual-reports` (contracheque, ano, avulsos, relatórios).

## Conventions

- Money is `rust_decimal::Decimal`, serialized as **string**; parse with `parseFloat` only for display.
- SQLite (`financas.db` in app_data_dir) is the source of truth; deterministic UUIDv5 → upsert.
- Backend layering: `commands → application → domain`; `infrastructure` for I/O. `domain` is Tauri/DB-free.
- Frontend: only `services/tauri.service.ts` calls `invoke`. Types mirror Rust DTOs in `types/api.types.ts`.

## Common commands

```bash
npm run tauri dev                 # run the app (hot-reload)
npm run tauri build               # installer for the current OS
npx vue-tsc --noEmit              # type-check
npm run test:run                  # Vitest
cd src-tauri && cargo test        # Rust tests
```
