import { afterEach, describe, expect, it } from "vitest";

import { isKeyEventOwnedByNestedLayer } from "@/lib/dialogEscape";

const NOT_PREVENTED = { defaultPrevented: false };
const PREVENTED = { defaultPrevented: true };

function mount(html: string): HTMLElement {
  const host = document.createElement("div");
  host.innerHTML = html;
  document.body.appendChild(host);
  return host;
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("isKeyEventOwnedByNestedLayer", () => {
  it("没有任何弹层时不拦截", () => {
    const host = mount(
      '<div id="sheet" role="dialog" aria-modal="true"></div>',
    );
    const sheet = host.querySelector<HTMLElement>("#sheet");

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(false);
  });

  it("内层已 preventDefault 时拦截（Radix DismissableLayer 的行为）", () => {
    const host = mount('<div id="sheet" role="dialog"></div>');
    const sheet = host.querySelector<HTMLElement>("#sheet");

    expect(isKeyEventOwnedByNestedLayer(PREVENTED, sheet)).toBe(true);
  });

  it("portal 到 body 的兄弟对话框算内层，即使没有 preventDefault", () => {
    const host = mount('<div id="sheet" role="dialog"></div>');
    const sheet = host.querySelector<HTMLElement>("#sheet");
    mount('<div role="dialog" data-state="open">确认修复</div>');

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(true);
  });

  it("alertdialog 同样算内层", () => {
    const host = mount('<div id="sheet" role="dialog"></div>');
    const sheet = host.querySelector<HTMLElement>("#sheet");
    mount('<div role="alertdialog" data-state="open"></div>');

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(true);
  });

  it("data-state=closed 的残留节点不算内层", () => {
    const host = mount('<div id="sheet" role="dialog"></div>');
    const sheet = host.querySelector<HTMLElement>("#sheet");
    mount('<div role="dialog" data-state="closed"></div>');

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(false);
  });

  it("抽屉自身的 role=dialog 外壳不算内层（否则 Esc 会被永久吃掉）", () => {
    const host = mount(
      '<div id="sheet" role="dialog" data-state="open"></div>',
    );
    const sheet = host.querySelector<HTMLElement>("#sheet");

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(false);
  });

  it("包住抽屉的祖先对话框是外层不是内层，不拦截", () => {
    const host = mount(
      '<div role="dialog" data-state="open"><div id="sheet"></div></div>',
    );
    const sheet = host.querySelector<HTMLElement>("#sheet");

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(false);
  });

  it("渲染在抽屉内部（非 portal）的对话框仍算内层", () => {
    const host = mount(
      '<div id="sheet" role="dialog"><div role="dialog" data-state="open"></div></div>',
    );
    const sheet = host.querySelector<HTMLElement>("#sheet");

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(true);
  });

  it("非模态弹层（下拉菜单 role=menu）不拦截，交给它自己的 preventDefault", () => {
    const host = mount('<div id="sheet" role="dialog"></div>');
    const sheet = host.querySelector<HTMLElement>("#sheet");
    mount('<div role="menu" data-state="open"></div>');

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, sheet)).toBe(false);
  });

  it("container 为 null 时保守拦截，不误关外层", () => {
    mount('<div role="dialog" data-state="open"></div>');

    expect(isKeyEventOwnedByNestedLayer(NOT_PREVENTED, null)).toBe(true);
  });
});
