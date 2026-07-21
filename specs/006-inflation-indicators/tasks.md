# Tasks: Indicadores de inflação (IPCA + inflação pessoal)

**Feature**: `006-inflation-indicators` | **Spec**: [spec.md](spec.md) | **Plan**: [plan.md](plan.md)

TDD (Constituição I): testes falham primeiro. `[P]` = paralelizável.

## Phase 1 — Setup

- [ ] T001 Adicionar `reqwest` (rustls-tls, json) ao `src-tauri/Cargo.toml` (feito) e criar `src-tauri/src/domain/inflation.rs` com os tipos `IpcaGroup`, `IpcaHeadline`, `InflationData`; reexportar em `domain/mod.rs`.

## Phase 2 — Foundational (cálculo puro, base de tudo)

- [ ] T002 [P] Testes que FALHAM para `map_category_to_group` e `compute_personal_inflation` em `domain/inflation.rs` (mapeamento por keyword; reponderação; categoria sem grupo → geral; sem gastos → pessoal==geral, diff==0; invariante de pesos).
- [ ] T003 Implementar `map_category_to_group` (keyword) e `compute_personal_inflation(categories, groups, general_month) -> (personal, diff)` — passar T002.

## Phase 3 — US3+US4: buscar (opt-in) e salvar localmente (P1/P2) 🎯 base de dados

**Meta**: baixar o IPCA sob demanda e persistir; funcionar offline.

- [ ] T004 [US4] `infrastructure/db.rs`: tabela `inflation_cache` (payload TEXT, fetched_at TEXT) + migração + `save_inflation_cache`/`load_inflation_cache`.
- [ ] T005 [US3] `infrastructure/ibge.rs`: `fetch_headline()` (agregado 1737 vars 63|2265|69) e `fetch_groups()` (agregado 7060 var 63, classificacao 315[all], filtrar os 9 grupos por nome) via reqwest; parse com serde_json; erros claros.
- [ ] T006 [US3] `commands/inflation.rs`: `fetch_ipca` (async — chama ibge, salva cache, retorna `InflationData`; erro preserva cache) e `get_inflation` (lê cache + calcula pessoal com as categorias atuais). Registrar em `lib.rs`.
- [ ] T007 [P] [US3] Teste (Rust) de parsing do JSON do IBGE em `infrastructure/ibge.rs` (fixtures de resposta 1737 e 7060 → headline/grupos corretos).

## Phase 4 — US1+US2: exibir IPCA + inflação pessoal (P1)

**Meta**: card no Ano (completo, com botão) e resumo no Mês.

- [ ] T008 [P] [US1] Tipos `IpcaGroup`/`IpcaHeadline`/`InflationData` em `src/types/api.types.ts` + `fetchIpca()`/`getInflation()` em `src/services/tauri.service.ts`.
- [ ] T009 [US1] `src/components/dashboard/InflationCard.vue`: IPCA (mês/ano/12m) + inflação pessoal + diferença + data da atualização + botão "Atualizar índices" (emite fetch) + estados vazio/erro/carregando; prop `compact`.
- [ ] T010 [US1] Integrar na `src/pages/YearPage.vue` (card completo, com botão) — carrega `getInflation` no mount.
- [ ] T011 [US2] Integrar na `src/pages/DashboardPage.vue` (resumo compacto).

## Phase 5 — Polish

- [ ] T012 [P] `cd src-tauri && cargo test` + `cargo clippy -p financas --all-targets -- -D warnings` verde.
- [ ] T013 [P] `npx vue-tsc --noEmit` + `npm run test:run` verde.
- [ ] T014 Validar quickstart no app: atualizar → ver IPCA + pessoal + data; reabrir offline → cache mantém; erro de rede → mensagem, cache preservado.

## Dependencies

- T001 → T002/T003.
- T003 (cálculo) + T004 (cache) + T005 (fetch) → T006 (comandos).
- T006 → US1/US2 (T008–T011).
- Polish por último.

## MVP

US3+US4+US1 (T001–T010): atualizar índices, salvar local, ver IPCA + inflação pessoal no Ano. Mês (T011) logo após.
