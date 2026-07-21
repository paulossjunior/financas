# Specification Quality Checklist: Importar extrato bancário do BTG

**Created**: 2026-07-21 · **Feature**: [spec.md](../spec.md)

## Content Quality
- [x] No implementation details · [x] User value · [x] Non-technical · [x] Mandatory sections done

## Requirement Completeness
- [x] No [NEEDS CLARIFICATION] · [x] Testável · [x] SC mensuráveis/agnósticas · [x] Cenários definidos
- [x] Edge cases (saldo diário, estorno, interno, multi-mês, inválido) · [x] Escopo delimitado · [x] Premissas

## Feature Readiness
- [x] FRs com critérios · [x] Fluxos cobertos · [x] Sem vazamento de implementação

## Notes
- Decidido: excluir automático (fatura/salário-com-contracheque/transferências internas); categorizar por regras do app + fallback BTG.
- /plan: onde na UI (nova tela vs botão no import atual) + como somar no painel (via pipeline manual).
