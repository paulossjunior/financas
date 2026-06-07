import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import MonthlyTrend from "@/components/dashboard/MonthlyTrend.vue";
import type { MonthlySnapshot } from "@/types/api.types";

vi.mock("vue-echarts", () => ({
  default: {
    name: "VChart",
    template: '<div class="vchart-stub"></div>',
    props: ["option", "autoresize"],
  },
}));
vi.mock("echarts/core", () => ({ use: vi.fn() }));
vi.mock("echarts/charts", () => ({ LineChart: {} }));
vi.mock("echarts/components", () => ({
  GridComponent: {},
  TooltipComponent: {},
  LegendComponent: {},
}));
vi.mock("echarts/renderers", () => ({ CanvasRenderer: {} }));

function makeSnapshot(month: string, cats: Record<string, string>): MonthlySnapshot {
  return {
    month,
    net_total: Object.values(cats).reduce((s, v) => (parseFloat(s) + parseFloat(v)).toFixed(2), "0.00"),
    categories: Object.entries(cats).map(([name, net_total]) => ({ name, net_total })),
  };
}

describe("MonthlyTrend", () => {
  it("renders without error with empty snapshots", () => {
    const wrapper = mount(MonthlyTrend, { props: { snapshots: [] } });
    expect(wrapper.find(".chart-container").exists()).toBe(true);
  });

  it("xAxis has one point per snapshot", () => {
    const snaps: MonthlySnapshot[] = [
      makeSnapshot("2026-04", { Alimentação: "400.00" }),
      makeSnapshot("2026-05", { Alimentação: "500.00" }),
    ];
    const wrapper = mount(MonthlyTrend, { props: { snapshots: snaps } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    expect(option.xAxis.data).toHaveLength(2);
  });

  it("formats month label correctly (Mai/2026)", () => {
    const snaps: MonthlySnapshot[] = [makeSnapshot("2026-05", { Alimentação: "500.00" })];
    const wrapper = mount(MonthlyTrend, { props: { snapshots: snaps } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    expect(option.xAxis.data[0]).toBe("Mai/2026");
  });

  it("creates one series per unique category", () => {
    const snaps: MonthlySnapshot[] = [
      makeSnapshot("2026-04", { Alimentação: "400.00", Transporte: "200.00" }),
      makeSnapshot("2026-05", { Alimentação: "500.00", Saúde: "100.00" }),
    ];
    const wrapper = mount(MonthlyTrend, { props: { snapshots: snaps } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    // 3 unique categories: Alimentação, Transporte, Saúde
    expect(option.series).toHaveLength(3);
  });

  it("fills missing category months with 0", () => {
    const snaps: MonthlySnapshot[] = [
      makeSnapshot("2026-04", { Alimentação: "400.00" }),
      makeSnapshot("2026-05", { Transporte: "200.00" }),
    ];
    const wrapper = mount(MonthlyTrend, { props: { snapshots: snaps } });
    const option = wrapper.findComponent({ name: "VChart" }).props("option") as any;
    const alimentacaoSeries = option.series.find((s: any) => s.name === "Alimentação");
    expect(alimentacaoSeries.data[0]).toBe(400);
    expect(alimentacaoSeries.data[1]).toBe(0); // missing in 2026-05
  });
});
