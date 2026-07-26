import { describe, expect, it } from "vitest";

import {
  isKnownProductError,
  translateProductError,
} from "./productAuthErrors";

describe("translateProductError", () => {
  it("unwraps the HTTP failure wrapper so entitlement denials are readable", () => {
    // authenticated_json returns "product_team_request_failed_<status>:<code>".
    // Without unwrapping, the user sees that raw token instead of the copy.
    expect(
      translateProductError(
        "product_team_request_failed_403:entitlement_inactive",
        "zh-CN",
      ),
    ).toBe(
      "团队功能目前为邀请制内测，当前账号所属团队尚未开通。个人使用的全部功能不受影响。",
    );
    expect(
      translateProductError(
        "product_team_request_failed_403:seat_limit_exceeded",
        "en-US",
      ),
    ).toContain("over its seat limit");
  });

  it("unwraps Tauri's invoke wrapper and the HTTP wrapper together", () => {
    expect(
      translateProductError(
        "Error invoking remote command 'team_key_sync': product_team_request_failed_403:capability_not_entitled",
        "en-US",
      ),
    ).toContain("does not include this feature");
  });

  it("never leaks a bare failure token when the server sent no code", () => {
    const zh = translateProductError(
      "product_team_request_failed_500",
      "zh-CN",
    );
    expect(zh).not.toContain("product_team_request_failed");
    expect(zh).toContain("500");

    const en = translateProductError(
      "product_team_request_failed_503",
      "en-US",
    );
    expect(en).toContain("HTTP 503");
  });

  it("keeps membership removal distinct from a lapsed entitlement", () => {
    // Both wipe local team keys, so both must say so — but the cause differs.
    const removed = translateProductError(
      "product_team_request_failed_403:forbidden",
      "zh-CN",
    );
    const lapsed = translateProductError(
      "product_team_request_failed_403:entitlement_inactive",
      "zh-CN",
    );
    expect(removed).toContain("已不在该团队");
    expect(removed).not.toBe(lapsed);
  });

  it("falls back to the raw code for anything unmapped", () => {
    expect(translateProductError("some_unmapped_code", "en-US")).toBe(
      "some_unmapped_code",
    );
  });

  it("picks language from the locale prefix", () => {
    expect(translateProductError("product_auth_login_cancelled", "zh-TW")).toBe(
      "登录已取消。",
    );
    expect(translateProductError("product_auth_login_cancelled", "de-DE")).toBe(
      "Login cancelled.",
    );
  });
});

describe("isKnownProductError", () => {
  it("recognizes wrapped codes, not just bare ones", () => {
    expect(isKnownProductError("entitlement_inactive")).toBe(true);
    expect(
      isKnownProductError(
        "product_team_request_failed_403:entitlement_inactive",
      ),
    ).toBe(true);
    expect(isKnownProductError("product_team_request_failed_500")).toBe(false);
    expect(isKnownProductError("totally_unknown")).toBe(false);
  });
});
