import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import type { CategoryGroup } from "@/types/api.types";

const GROUP: CategoryGroup = { name: "Alimentação", keywords: ["IFOOD", "RESTAURANTE"], priority: 10 };
const ALL_KWS = [
  { keyword: "IFOOD", category: "Alimentação" },
  { keyword: "RESTAURANTE", category: "Alimentação" },
  { keyword: "UBER", category: "Transporte" },
];

async function mountEditor(group = GROUP, allKeywords = ALL_KWS) {
  const { default: CategoryGroupEditor } = await import("@/components/settings/CategoryGroupEditor.vue");
  return mount(CategoryGroupEditor, { props: { group, allKeywords } });
}

describe("CategoryGroupEditor", () => {
  it("renders category name and keyword chips", async () => {
    const wrapper = await mountEditor();
    expect(wrapper.text()).toContain("Alimentação");
    expect(wrapper.text()).toContain("IFOOD");
    expect(wrapper.text()).toContain("RESTAURANTE");
  });

  it("add keyword emits update event with new keyword appended", async () => {
    const wrapper = await mountEditor();
    const input = wrapper.find("[data-testid='keyword-input']");
    await input.setValue("PIZZA");
    await wrapper.find("[data-testid='add-keyword-btn']").trigger("click");
    const emitted = wrapper.emitted("update") as any;
    expect(emitted).toBeTruthy();
    const updatedGroup = emitted[0][0] as CategoryGroup;
    expect(updatedGroup.keywords).toContain("PIZZA");
  });

  it("remove keyword chip emits update event with keyword removed", async () => {
    const wrapper = await mountEditor();
    await wrapper.find("[data-testid='remove-keyword-btn']").trigger("click");
    const emitted = wrapper.emitted("update") as any;
    expect(emitted).toBeTruthy();
    const updatedGroup = emitted[0][0] as CategoryGroup;
    expect(updatedGroup.keywords).not.toContain("IFOOD");
  });

  it("shows conflict warning when keyword exists in other category", async () => {
    const wrapper = await mountEditor();
    const input = wrapper.find("[data-testid='keyword-input']");
    await input.setValue("UBER");
    await input.trigger("input");
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("Transporte");
  });

  it("emits delete event when delete button clicked", async () => {
    const wrapper = await mountEditor();
    await wrapper.find("[data-testid='delete-group-btn']").trigger("click");
    expect(wrapper.emitted("delete")).toBeTruthy();
  });
});
