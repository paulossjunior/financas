---
name: nielsen-heuristics
description: Nielsen's 10 usability heuristics for UI/UX. Load when designing, building, or reviewing any screen, component, flow, or error message in this app — use the per-heuristic checklist to catch gaps before shipping.
---

# Nielsen's 10 Usability Heuristics

Apply these whenever you add or change UI in Finanças (pages under `src/pages/`,
components under `src/components/`, flows, empty states, error messages). Review a
change against every heuristic; note which apply and fix the gaps.

## The heuristics + what to check

1. **Visibility of system status** — the system always keeps users informed.
   - [ ] Loading/saving/importing show a spinner or "…" state (never a frozen UI).
   - [ ] Actions confirm their result (toast/message: "N recategorizados", "criada").
   - [ ] The current context is visible (active nav tab, selected month, active filter badge).

2. **Match between system and the real world** — speak the user's language.
   - [ ] Plain pt-BR, not jargon (say "conta fixa"/"crédito", not "recurring flag").
   - [ ] Money as `R$ 1.234,56`; dates human (`14/jul/2026`); real-world concepts (fatura, contracheque, extrato).

3. **User control and freedom** — clear exits, undo/redo.
   - [ ] Modals/dialogs have Cancel + Esc + backdrop-click to close.
   - [ ] Destructive/edit actions are reversible or re-doable (remove keyword → item returns to Outros to re-map; edit/remove lançamento).
   - [ ] Filters can be cleared; a wrong choice is easy to back out of.

4. **Consistency and standards** — same things look/behave the same.
   - [ ] Reuse the emerald `--clr-*` tokens, existing button/chip/card classes, tab pattern.
   - [ ] Same term for the same concept across screens (Despesas & Receitas, Fixos & Renda, Categorias).
   - [ ] Icons/colors mean the same everywhere (green = receita/positivo, vermelho = negativo, âmbar = alerta/fixo).

5. **Error prevention** — prevent problems before they happen.
   - [ ] Validate before submit (empty name, duplicate category, amount > 0, valid month).
   - [ ] Confirm irreversible actions; disable buttons while saving.
   - [ ] Avoid states that can double-count (anti-duplicação extrato↔fixo manual).

6. **Recognition rather than recall** — show options, don't make users remember.
   - [ ] Suggestions/datalists for categories & keywords; placeholders showing the computed baseline.
   - [ ] Origin/status chips (Extrato/Fatura/Manual, realizado/estimado) instead of implicit rules.
   - [ ] Visible where things came from and where they'll go.

7. **Flexibility and efficiency of use** — serve novices and power users.
   - [ ] Sensible defaults (recorrente marcado, mês atual, baseline 3m) with overrides.
   - [ ] Shortcuts: Enter to submit, quick filters ("só recorrentes"), one-click "Marcar recorrente" from a suggestion.

8. **Aesthetic and minimalist design** — no clutter; hierarchy.
   - [ ] Primary numbers stand out; secondary muted; zeros de-emphasized.
   - [ ] Group related info; avoid walls of equal cards; progressive disclosure (vigência only when recorrente).

9. **Help users recognize, diagnose, recover from errors** — clear error messages.
   - [ ] Errors in plain language, say what happened and how to fix (App.vue `toPortuguese`, service `mapError`).
   - [ ] Never a raw code/stack; point to the next step.

10. **Help and documentation** — help is available when needed.
    - [ ] Inline hints explain non-obvious concepts (vigência, valor base, "recém-saído").
    - [ ] Manual/site cover the feature; link to it where relevant.

## How to use in a review

For the change under review, go heuristic by heuristic: mark ✅ implemented, ⚠️ partial, or ❌ missing, with a concrete file/line and a one-line fix. Prioritize ❌ that block a task and ⚠️ on money/error flows (Data Integrity + trust matter most here).
