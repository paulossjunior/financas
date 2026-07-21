import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import MonthGroup from "@/components/history/MonthGroup.vue";
import type { MonthGroup as MonthGroupType, InvoiceInfo } from "@/types/api.types";

function makeInvoice(overrides: Partial<InvoiceInfo> = {}): InvoiceInfo {
  return {
    id: "inv-1",
    filename: "2026-05-fatura.xlsx",
    month: "2026-05",
    row_count: 10,
    imported_at: "2026-05-01T10:00:00",
    ...overrides,
  };
}

function makeGroup(overrides: Partial<MonthGroupType> = {}): MonthGroupType {
  return {
    month: "2026-05",
    label: "Maio 2026",
    invoices: [makeInvoice()],
    net_total: "1500.00",
    invoice_count: 1,
    ...overrides,
  };
}

describe("MonthGroup", () => {
  it("renders month label", () => {
    const wrapper = mount(MonthGroup, { props: { group: makeGroup(), isActive: false } });
    expect(wrapper.text()).toContain("Maio 2026");
  });

  it("renders net_total formatted as R$", () => {
    const wrapper = mount(MonthGroup, { props: { group: makeGroup({ net_total: "1500.00" }), isActive: false } });
    expect(wrapper.text()).toContain("1.500");
  });

  it("renders invoice_count badge", () => {
    const wrapper = mount(MonthGroup, { props: { group: makeGroup({ invoice_count: 3 }), isActive: false } });
    expect(wrapper.text()).toContain("3");
  });

  it("renders one InvoiceRow per invoice in group", () => {
    const group = makeGroup({
      invoices: [
        makeInvoice({ id: "a", filename: "a.xlsx" }),
        makeInvoice({ id: "b", filename: "b.xlsx" }),
      ],
      invoice_count: 2,
    });
    const wrapper = mount(MonthGroup, { props: { group, isActive: false } });
    expect(wrapper.findAll("[data-testid='remove-btn']")).toHaveLength(2);
  });

  it("emits 'filter-month' with month string when 'Ver dashboard' button clicked", async () => {
    const wrapper = mount(MonthGroup, { props: { group: makeGroup({ month: "2026-05" }), isActive: false } });
    await wrapper.find("[data-testid='filter-btn']").trigger("click");
    expect(wrapper.emitted("filter-month")).toBeTruthy();
    expect(wrapper.emitted("filter-month")![0]).toEqual(["2026-05"]);
  });

  it("emits 'remove-invoice' with id when InvoiceRow emits 'remove'", async () => {
    const wrapper = mount(MonthGroup, { props: { group: makeGroup(), isActive: false } });
    await wrapper.find("[data-testid='remove-btn']").trigger("click");
    expect(wrapper.emitted("remove-invoice")).toBeTruthy();
    expect(wrapper.emitted("remove-invoice")![0]).toEqual(["inv-1"]);
  });

  it("shows '—' for net_total when null (unknown month group)", () => {
    const wrapper = mount(MonthGroup, {
      props: { group: makeGroup({ net_total: null, label: "Mês desconhecido" }), isActive: false },
    });
    expect(wrapper.text()).toContain("—");
  });
});
