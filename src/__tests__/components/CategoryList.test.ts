import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import type { CategoryGroup } from "@/types/api.types";

const GROUPS: CategoryGroup[] = [
  { name: "Alimentação", keywords: ["IFOOD"], priority: 10 },
  { name: "Transporte", keywords: ["UBER"], priority: 20 },
];

async function mountCategoryList(groups = GROUPS) {
  const { default: CategoryList } = await import("@/components/settings/CategoryList.vue");
  return mount(CategoryList, { props: { groups } });
}

describe("CategoryList", () => {
  it("renders one row per CategoryGroup", async () => {
    const wrapper = await mountCategoryList();
    const rows = wrapper.findAll("[data-testid='category-row']");
    expect(rows).toHaveLength(2);
    const inputs = wrapper.findAll("[data-testid='category-name-input']");
    expect((inputs[0].element as HTMLInputElement).value).toBe("Alimentação");
    expect((inputs[1].element as HTMLInputElement).value).toBe("Transporte");
  });

  it("add button emits add-category event", async () => {
    const wrapper = await mountCategoryList();
    await wrapper.find("[data-testid='add-category-btn']").trigger("click");
    expect(wrapper.emitted("add-category")).toBeTruthy();
  });

  it("delete button emits delete-category with category name", async () => {
    const wrapper = await mountCategoryList();
    await wrapper.find("[data-testid='delete-category-btn']").trigger("click");
    expect(wrapper.emitted("delete-category")?.[0]).toEqual(["Alimentação"]);
  });

  it("rename input emits rename-category with old and new name", async () => {
    const wrapper = await mountCategoryList();
    const input = wrapper.find("[data-testid='category-name-input']");
    await input.setValue("Comida");
    await input.trigger("change");
    expect(wrapper.emitted("rename-category")?.[0]).toEqual(["Alimentação", "Comida"]);
  });
});
