import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import InvoiceRow from "@/components/history/InvoiceRow.vue";
import type { InvoiceInfo } from "@/types/api.types";

function makeInvoice(overrides: Partial<InvoiceInfo> = {}): InvoiceInfo {
  return {
    id: "abc-123",
    bank: "BTG",
    filename: "2026-05-fatura-btg.xlsx",
    month: "2026-05",
    row_count: 42,
    imported_at: "2026-05-10T14:30:00",
    ...overrides,
  };
}

describe("InvoiceRow", () => {
  it("renders invoice filename", () => {
    const wrapper = mount(InvoiceRow, { props: { invoice: makeInvoice() } });
    expect(wrapper.text()).toContain("2026-05-fatura-btg.xlsx");
  });

  it("renders the bank badge", () => {
    const wrapper = mount(InvoiceRow, {
      props: { invoice: makeInvoice({ bank: "Santander" }) },
    });
    expect(wrapper.get('[data-testid="invoice-bank"]').text()).toBe("Santander");
  });

  it("renders row_count", () => {
    const wrapper = mount(InvoiceRow, { props: { invoice: makeInvoice({ row_count: 42 }) } });
    expect(wrapper.text()).toContain("42");
  });

  it("renders imported_at formatted as dd/mm/yyyy", () => {
    const wrapper = mount(InvoiceRow, {
      props: { invoice: makeInvoice({ imported_at: "2026-05-10T14:30:00" }) },
    });
    expect(wrapper.text()).toContain("10/05/2026");
  });

  it("remove button is present and enabled", () => {
    const wrapper = mount(InvoiceRow, { props: { invoice: makeInvoice() } });
    const btn = wrapper.find("[data-testid='remove-btn']");
    expect(btn.exists()).toBe(true);
    expect((btn.element as HTMLButtonElement).disabled).toBe(false);
  });

  it("emits 'remove' event with invoiceId when remove button clicked", async () => {
    const wrapper = mount(InvoiceRow, { props: { invoice: makeInvoice({ id: "abc-123" }) } });
    await wrapper.find("[data-testid='remove-btn']").trigger("click");
    expect(wrapper.emitted("remove")).toBeTruthy();
    expect(wrapper.emitted("remove")![0]).toEqual(["abc-123"]);
  });
});
