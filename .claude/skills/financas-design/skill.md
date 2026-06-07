---
name: financas-design
description: Design system and UI improvement guide for the Financas personal finance dashboard
compatibility: Financas Tauri/Vue 3 project
metadata:
  author: paulossjunior
  source: local
---

# Financas Design System

Apply this skill when asked to improve, review, or redesign any UI component in the Financas app.

## Design Principles

1. **Data-first**: Numbers must be readable at a glance. Hierarchy: total → category → transaction.
2. **Calm palette**: Finance apps should feel trustworthy, not exciting. No loud colors.
3. **Density balance**: Personal tool — show more data, less chrome. No empty states if data exists.
4. **Color as signal**: Green = positive balance, Red = expense/negative, Purple/Blue = highlight/accent. Never decorative only.

## Color Palette

```css
/* Backgrounds */
--bg-app:       #f7fafc;   /* page background */
--bg-card:      #ffffff;   /* card/panel */
--bg-subtle:    #f0f4f8;   /* subtle section bg */

/* Text */
--text-primary:   #1a202c; /* headings */
--text-secondary: #4a5568; /* labels, metadata */
--text-muted:     #718096; /* hints, empty states */

/* Accents */
--accent-blue:   #3182ce;  /* primary action, nav active */
--accent-purple: #6b46c1;  /* biggest spend highlight */
--accent-green:  #276749;  /* positive amounts */
--accent-red:    #c53030;  /* negative/expense amounts */

/* Chart palette (ordered, ECharts) */
--chart-1: #6366f1;  /* indigo */
--chart-2: #10b981;  /* emerald */
--chart-3: #f59e0b;  /* amber */
--chart-4: #ef4444;  /* red */
--chart-5: #8b5cf6;  /* violet */
--chart-6: #06b6d4;  /* cyan */
--chart-7: #f97316;  /* orange */
--chart-8: #84cc16;  /* lime */
```

## Typography Scale

```css
--text-xs:   0.75rem;   /* metadata, tags */
--text-sm:   0.875rem;  /* table cells, labels */
--text-base: 1rem;      /* body, card content */
--text-lg:   1.125rem;  /* card titles, section headers */
--text-xl:   1.5rem;    /* page title */
--text-2xl:  2rem;      /* big number display (total) */

font-weight-normal:  400
font-weight-medium:  500
font-weight-semibold: 600
font-weight-bold:    700
```

## Spacing System

Use multiples of 4px (0.25rem):
- `xxs`: 0.25rem (4px) — icon gap, tight labels
- `xs`:  0.5rem  (8px) — inline spacing
- `sm`:  0.75rem (12px) — compact padding
- `md`:  1rem    (16px) — default padding
- `lg`:  1.5rem  (24px) — section spacing
- `xl`:  2rem    (32px) — page padding

## Component Patterns

### BiggestSpendBanner
- Gradient: `linear-gradient(135deg, #6366f1 0%, #6b46c1 100%)`
- Large category name: `--text-2xl`, bold
- Amount: `--text-xl`, semibold, white
- Percentage chip: small pill, semi-transparent white bg

### Summary Bar (KPI row)
- 3–4 KPI cards in a row
- Each card: label (xs, muted), value (2xl, bold), optional delta
- Subtle border or shadow, not full cards — keep it light

### Category Chart (donut)
- Radius: 45%–72% (wider donut = more modern)
- Legend below chart, 2-column grid
- Tooltip: name + amount + percentage

### Category Ranking (horizontal bar)
- Bars left-to-right, label on left (truncated at 120px)
- Highest bar: accent purple, others: chart palette
- Show value at end of bar

### Transactions Table
- Row height: 44px minimum (touch-friendly)
- Alternating row bg: white / `--bg-subtle`
- Amount column: right-aligned, monospace font
- Date: fixed-width column, `--text-sm`

### Cards / Panels
```css
.card {
  background: var(--bg-card);
  border-radius: 12px;
  border: 1px solid #e2e8f0;
  padding: 1.25rem 1.5rem;
  box-shadow: 0 1px 3px rgba(0,0,0,0.06);
}
```

## Layout Grid

Dashboard page:
```
[ Summary KPI bar — full width ]
[ BiggestSpendBanner — full width ]
[ Donut chart — 50% ] [ Ranking bar — 50% ]
[ Top Transactions table — full width ]
```

History/Listagem page:
```
[ Month group header: Mês/Ano | Total | N faturas ]
  [ Invoice row: filename | transactions | imported date | remove btn ]
  [ Invoice row: ... ]
[ Month group header: ... ]
```

## ECharts Color Override

Apply consistent palette in every chart option:
```js
color: ['#6366f1', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#06b6d4', '#f97316', '#84cc16']
```

## Common Issues to Fix

- `R$ NaN` in amounts → always guard: `parseFloat(val) || 0` before formatting
- Chart tooltip using raw decimal string → parse before `toLocaleString`
- Missing `pt-BR` locale on amount formatting → always pass `{ style: 'currency', currency: 'BRL' }`
- Chart not resizing on window resize → ensure `autoresize` prop set on `VChart`

## When Applying Design Changes

1. Update CSS custom properties in `src/App.vue` global `<style>` block
2. Update component `<style scoped>` to use variables
3. Update ECharts `option` color array in each chart component
4. Screenshot before/after using Playwright for validation
