import { describe, it, expect } from "vitest";
import type { CategoryGroup } from "@/types/api.types";

async function getDetectConflict() {
  const { detectConflict } = await import("@/utils/category-conflict");
  return detectConflict;
}

const GROUPS: CategoryGroup[] = [
  { name: "Alimentação", keywords: ["IFOOD", "RESTAURANTE"], priority: 10 },
  { name: "Transporte", keywords: ["UBER", "99"], priority: 20 },
];

describe("detectConflict", () => {
  it("returns null when keyword is not used by any category", async () => {
    const detectConflict = await getDetectConflict();
    expect(detectConflict("NETFLIX", "Lazer", GROUPS)).toBeNull();
  });

  it("returns existing category name when keyword is duplicate", async () => {
    const detectConflict = await getDetectConflict();
    expect(detectConflict("IFOOD", "Lazer", GROUPS)).toBe("Alimentação");
  });

  it("is case-insensitive", async () => {
    const detectConflict = await getDetectConflict();
    expect(detectConflict("ifood", "Lazer", GROUPS)).toBe("Alimentação");
    expect(detectConflict("IFOOD", "lazer", GROUPS)).toBe("Alimentação");
  });

  it("ignores current category when checking for conflict", async () => {
    const detectConflict = await getDetectConflict();
    // "IFOOD" is in "Alimentação" — editing "Alimentação" should NOT flag itself
    expect(detectConflict("IFOOD", "Alimentação", GROUPS)).toBeNull();
  });
});
