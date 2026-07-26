<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan
at specs/014-banestes-statement-adapter/plan.md
<!-- SPECKIT END -->

## Project overview

Desktop personal-finance app (Tauri v2 + Rust + Vue 3 + SQLite). Imports BTG
card invoices (.xlsx, possibly encrypted) and SouGov.br payslips (PDF); shows a
monthly dashboard, an annual view, and PDF reports. 100% local, no network.

- **README**: [README.md](README.md) — features, install, dev, build, CI/CD.
- **Architecture**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — layers, data/money model, flows, DB, commands.
- **Maintenance**: [docs/MAINTENANCE.md](docs/MAINTENANCE.md) — invariants, how-to recipes, DB, release, gotchas.
- **Specs** (spec-kit): `specs/001-credit-card-dashboard`, `002-monthly-invoice-list`,
  `003-custom-categories`, `004-payslips-annual-reports` (contracheque, ano, avulsos, relatórios).

## Real files (never in git)

Real financial documents live **outside the repo**, in `/Users/paulossjunior/Documents/casa/`:

| Folder | Content |
|---|---|
| `casa/extratos/` | bank statements (Banestes `.pdf`, BTG `.xls`) |
| `casa/faturas/` | BTG card invoices (`.xlsx`, some encrypted) |
| `casa/contracheque/` | SouGov.br payslips (`.pdf`) |

Rules:

- Read test files from there — **never copy one into the repo**, not even temporarily.
- A repo fixture is always **anonymized**: fictitious holder/counterparties/account,
  real dates and amounts (see `tests/fixtures/banestes_extrato.txt`). Derive it by hand
  from a `casa/` file; the original stays put.
- Never paste a real holder name, account number, or counterparty into code, tests, specs,
  commit messages, or docs.
- Scratch scripts that read these files go in the session scratchpad, not in `src-tauri/`.
  If one has to live in the crate to compile (e.g. `src-tauri/examples/`), delete it in the
  same turn and confirm with `git status`.

## Conventions

- Money is `rust_decimal::Decimal`, serialized as **string**; parse with `parseFloat` only for display.
- SQLite (`financas.db` in app_data_dir) is the source of truth; deterministic UUIDv5 → upsert.
- Backend layering: `commands → application → domain`; `infrastructure` for I/O. `domain` is Tauri/DB-free.
- Frontend: only `services/tauri.service.ts` calls `invoke`. Types mirror Rust DTOs in `types/api.types.ts`.
- **UI/UX**: when designing, building, or reviewing any screen/component/flow/error, load and apply the **`nielsen-heuristics`** skill (`.claude/skills/nielsen-heuristics/SKILL.md`) — review the change against its 10-point checklist before shipping.

## Common commands

```bash
npm run tauri dev                 # run the app (hot-reload)
npm run tauri build               # installer for the current OS
npx vue-tsc --noEmit              # type-check
npm run test:run                  # Vitest
cd src-tauri && cargo test        # Rust tests
```
