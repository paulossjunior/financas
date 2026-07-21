import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";

const CATEGORIES = ["Alimentação", "Transporte", "Educação", "Outros"];

interface OverrideProps {
  transactionId: string;
  currentCategory: string;
  availableCategories: string[];
  hasOverride: boolean;
}

async function mountOverride(props: OverrideProps) {
  const { default: TransactionCategoryOverride } = await import(
    "@/components/settings/TransactionCategoryOverride.vue"
  );
  return mount(TransactionCategoryOverride, { props });
}

describe("TransactionCategoryOverride", () => {
  it("renders current category", async () => {
    const wrapper = await mountOverride({
      transactionId: "tx-1",
      currentCategory: "Alimentação",
      availableCategories: CATEGORIES,
      hasOverride: false,
    });
    expect(wrapper.text()).toContain("Alimentação");
  });

  it("dropdown shows all available category names", async () => {
    const wrapper = await mountOverride({
      transactionId: "tx-1",
      currentCategory: "Alimentação",
      availableCategories: CATEGORIES,
      hasOverride: false,
    });
    const options = wrapper.findAll("option");
    const optionTexts = options.map((o) => o.text());
    expect(optionTexts).toContain("Educação");
    expect(optionTexts).toContain("Transporte");
  });

  it("selecting different category emits override event", async () => {
    const wrapper = await mountOverride({
      transactionId: "tx-1",
      currentCategory: "Alimentação",
      availableCategories: CATEGORIES,
      hasOverride: false,
    });
    const select = wrapper.find("select");
    await select.setValue("Educação");
    const emitted = wrapper.emitted("override") as any;
    expect(emitted).toBeTruthy();
    expect(emitted[0]).toEqual(["tx-1", "Educação"]);
  });

  it("shows override indicator when hasOverride is true", async () => {
    const wrapper = await mountOverride({
      transactionId: "tx-1",
      currentCategory: "Educação",
      availableCategories: CATEGORIES,
      hasOverride: true,
    });
    expect(wrapper.find("[data-testid='override-badge']").exists()).toBe(true);
  });

  it("remove-override button emits removeOverride event when hasOverride is true", async () => {
    const wrapper = await mountOverride({
      transactionId: "tx-1",
      currentCategory: "Educação",
      availableCategories: CATEGORIES,
      hasOverride: true,
    });
    await wrapper.find("[data-testid='remove-override-btn']").trigger("click");
    expect(wrapper.emitted("removeOverride")?.[0]).toEqual(["tx-1"]);
  });
});
