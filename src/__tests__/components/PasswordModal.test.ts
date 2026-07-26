import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import PasswordModal from "@/components/import/PasswordModal.vue";

function mountModal(props: Partial<InstanceType<typeof PasswordModal>["$props"]> = {}) {
  return mount(PasswordModal, {
    props: { open: true, loading: false, error: null, bank: null, ...props },
  });
}

describe("PasswordModal", () => {
  it("names the bank being asked — banks have different passwords", () => {
    const wrapper = mountModal({ bank: "Santander" });
    expect(wrapper.get('[data-testid="pw-title"]').text()).toContain("Santander");
    expect(wrapper.get('[data-testid="pw-input"]').attributes("placeholder")).toContain(
      "Santander"
    );
  });

  it("falls back to a generic label when the bank is unknown", () => {
    const wrapper = mountModal({ bank: null });
    expect(wrapper.get('[data-testid="pw-title"]').text()).toBe("Senha da fatura");
  });

  it("emits submit with password and remember, disabled while empty", async () => {
    const wrapper = mountModal({ bank: "BTG" });
    const submit = wrapper.get('[data-testid="pw-submit"]');
    expect(submit.attributes("disabled")).toBeDefined();

    await wrapper.get('[data-testid="pw-input"]').setValue("senha-btg");
    await submit.trigger("click");
    expect(wrapper.emitted("submit")?.[0]).toEqual(["senha-btg", true]);
  });

  it("shows the error fed by the parent", () => {
    const wrapper = mountModal({ error: "Senha incorreta. Tente novamente." });
    expect(wrapper.get('[data-testid="pw-error"]').text()).toContain("Senha incorreta");
  });
});
