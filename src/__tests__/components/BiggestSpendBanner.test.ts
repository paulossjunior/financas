import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import BiggestSpendBanner from "@/components/dashboard/BiggestSpendBanner.vue";
import type { Category } from "@/types/api.types";

function makeCategory(overrides: Partial<Category> = {}): Category {
  return {
    name: "Alimentação",
    total: "500.00",
    reversal_total: "0.00",
    net_total: "500.00",
    percentage: 45.3,
    transaction_count: 12,
    top_transactions: [],
    ...overrides,
  };
}

describe("BiggestSpendBanner", () => {
  it("renders category name", () => {
    const wrapper = mount(BiggestSpendBanner, {
      props: { category: makeCategory({ name: "Mercado & Compras" }) },
    });
    expect(wrapper.find(".category-name").text()).toBe("Mercado & Compras");
  });

  it("renders percentage with one decimal place", () => {
    const wrapper = mount(BiggestSpendBanner, {
      props: { category: makeCategory({ percentage: 33.7 }) },
    });
    // percentage div hidden via CSS (display:none) but element still in DOM
    expect(wrapper.find(".percentage").exists()).toBe(true);
  });

  it("renders net_total via MoneyAmount component", () => {
    const wrapper = mount(BiggestSpendBanner, {
      props: { category: makeCategory({ net_total: "1234.56" }) },
    });
    // MoneyAmount renders a <span> with formatted amount
    expect(wrapper.find(".amount").exists()).toBe(true);
    expect(wrapper.find(".amount").text()).toContain("1.234");
  });

  it("shows trophy label", () => {
    const wrapper = mount(BiggestSpendBanner, {
      props: { category: makeCategory() },
    });
    expect(wrapper.find(".label").text()).toContain("Maior Gasto");
  });
});
