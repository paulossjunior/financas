import type { CategoryGroup } from "@/types/api.types";

export function detectConflict(
  keyword: string,
  currentCategoryName: string,
  groups: CategoryGroup[]
): string | null {
  const kw = keyword.toUpperCase();
  for (const g of groups) {
    if (g.name.toUpperCase() === currentCategoryName.toUpperCase()) continue;
    if (g.keywords.some((k) => k.toUpperCase() === kw)) return g.name;
  }
  return null;
}
