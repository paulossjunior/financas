import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import ImportButton from "@/components/import/ImportButton.vue";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
}));

import { open } from "@tauri-apps/plugin-dialog";
const mockOpen = vi.mocked(open);

describe("ImportButton", () => {
  beforeEach(() => {
    mockOpen.mockReset();
  });

  it("renders import button", () => {
    const wrapper = mount(ImportButton);
    expect(wrapper.find("button.import-btn").exists()).toBe(true);
    expect(wrapper.text()).toContain("Importar Faturas");
  });

  it("emits import-requested with paths when dialog returns files", async () => {
    mockOpen.mockResolvedValue(["/home/user/fatura1.xlsx", "/home/user/fatura2.xlsx"]);
    const wrapper = mount(ImportButton);

    await wrapper.find("button").trigger("click");
    await vi.waitUntil(() => wrapper.emitted("import-requested"));

    const emitted = wrapper.emitted("import-requested") as string[][];
    expect(emitted).toHaveLength(1);
    expect(emitted[0][0]).toEqual(["/home/user/fatura1.xlsx", "/home/user/fatura2.xlsx"]);
  });

  it("emits import-requested when dialog returns single string", async () => {
    mockOpen.mockResolvedValue("/home/user/fatura.xlsx");
    const wrapper = mount(ImportButton);

    await wrapper.find("button").trigger("click");
    await vi.waitUntil(() => wrapper.emitted("import-requested"));

    const emitted = wrapper.emitted("import-requested") as string[][];
    expect(emitted[0][0]).toEqual(["/home/user/fatura.xlsx"]);
  });

  it("does not emit when dialog returns null (cancelled)", async () => {
    mockOpen.mockResolvedValue(null);
    const wrapper = mount(ImportButton);

    await wrapper.find("button").trigger("click");
    await new Promise((r) => setTimeout(r, 50));

    expect(wrapper.emitted("import-requested")).toBeUndefined();
  });

  it("disables button while loading", async () => {
    let resolve: (v: string[]) => void;
    mockOpen.mockReturnValue(new Promise((r) => (resolve = r)));
    const wrapper = mount(ImportButton);

    wrapper.find("button").trigger("click");
    await wrapper.vm.$nextTick();

    expect(wrapper.find("button").attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain("Importando");

    resolve!([]);
    await vi.waitUntil(() => !wrapper.find("button").attributes("disabled"));
  });
});
