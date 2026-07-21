import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import CategoryRanking from "@/components/dashboard/CategoryRanking.vue";
import type { Category } from "@/types/api.types";

vi.mock("vue-echarts", () => ({
  default: {
    name: "VChart",
    template: '<div class="vchart-stub"></div>',
    props: ["option", "autoresize"],
  },
}));
vi.mock("echarts/core", () => ({ use: vi.fn() }));
vi.mock("echarts/charts", () => ({ BarChart: {} }));
vi.mock("echarts/components", () => ({ GridComponent: {}, TooltipComponent: {} }));
vi.mock("echarts/renderers", () => ({ CanvasRenderer: {} }));

function makeCategory(name: string, net_total: string, pct: number): Category {
  return {
    name,
    total: net_total,
    reversal_total: "0.00",
    net_total,
    percentage: pct,
    transaction_count: 1,
    top_transactions: [],
  };
}

describe("CategoryRanking", () => {
  it("renders without error with empty categories", () => {
    const wrapper = mount(CategoryRanking, { props: { categories: [] } });
    expect(wrapper.find(".chart-container").exists()).toBe(true);
    expect(wrapper.text()).toContain("Ranking de Categorias");
  });

  it("sorts categories ascending by net_total in yAxis", () => {
    const cats: Category[] = [
      makeCategory("Saúde", "200.00", 20),
      makeCategory("Alimentação", "500.00", 50),
      makeCategory("Transporte", "300.00", 30),
    ];
    const wrapper = mount(CategoryRanking, { props: { categories: cats } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;

    // Ascending sort: lowest first
    expect(option.yAxis.data[0]).toBe("Saúde");
    expect(option.yAxis.data[1]).toBe("Transporte");
    expect(option.yAxis.data[2]).toBe("Alimentação");
  });

  it("last bar (highest category) gets accent color", () => {
    const cats: Category[] = [
      makeCategory("Saúde", "200.00", 20),
      makeCategory("Alimentação", "500.00", 50),
    ];
    const wrapper = mount(CategoryRanking, { props: { categories: cats } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    const data = option.series[0].data;
    expect(data[data.length - 1].itemStyle.color).toBe("#0078d4");
    expect(data[0].itemStyle.color).toBe("#c7e0f4");
  });

  it("series data length matches categories prop length", () => {
    const cats = [
      makeCategory("A", "100.00", 33),
      makeCategory("B", "200.00", 67),
    ];
    const wrapper = mount(CategoryRanking, { props: { categories: cats } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    expect(option.series[0].data).toHaveLength(cats.length);
  });
});
