import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import CategoryChart from "@/components/dashboard/CategoryChart.vue";
import type { Category } from "@/types/api.types";

// VChart requires canvas — stub it in jsdom/happy-dom
vi.mock("vue-echarts", () => ({
  default: {
    name: "VChart",
    template: '<div class="vchart-stub"></div>',
    props: ["option", "autoresize"],
  },
}));

vi.mock("echarts/core", () => ({ use: vi.fn() }));
vi.mock("echarts/charts", () => ({ PieChart: {} }));
vi.mock("echarts/components", () => ({
  TitleComponent: {},
  TooltipComponent: {},
  LegendComponent: {},
}));
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

describe("CategoryChart", () => {
  it("renders without error when categories is empty", () => {
    const wrapper = mount(CategoryChart, { props: { categories: [] } });
    expect(wrapper.find(".chart-container").exists()).toBe(true);
  });

  it("maps categories to chart series data correctly", async () => {
    const cats: Category[] = [
      makeCategory("Alimentação", "500.00", 50),
      makeCategory("Transporte", "300.00", 30),
      makeCategory("Saúde", "200.00", 20),
    ];

    const wrapper = mount(CategoryChart, { props: { categories: cats } });
    const vchartStub = wrapper.findComponent({ name: "VChart" });
    const option = vchartStub.props("option") as any;

    expect(option.series).toHaveLength(1);
    expect(option.series[0].data).toHaveLength(3);
    expect(option.series[0].data[0]).toMatchObject({
      name: "Alimentação",
      value: "500.00",
      percentage: 50,
    });
  });

  it("series data length matches categories prop length", () => {
    const cats = [
      makeCategory("A", "100.00", 33),
      makeCategory("B", "200.00", 67),
    ];
    const wrapper = mount(CategoryChart, { props: { categories: cats } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    expect(option.series[0].data).toHaveLength(cats.length);
  });
});
