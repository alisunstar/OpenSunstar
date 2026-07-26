/**
 * Product auth / team error code → user-friendly message mapping.
 *
 * Rust commands return snake_case error codes (e.g. "product_auth_control_plane_not_configured").
 * This module translates them into localized, actionable messages before display.
 */

const ERROR_MESSAGES: Record<string, { zh: string; en: string }> = {
  // --- Entitlement gate ---
  // The control plane denies every paid key-vault route with 403 unless the
  // organization holds a live entitlement. Each message states plainly that
  // individual use is untouched — that separation is the product's red line,
  // not a marketing line.
  entitlement_inactive: {
    zh: "团队功能目前为邀请制内测，当前账号所属团队尚未开通。个人使用的全部功能不受影响。",
    en: "Team features are in invite-only beta and this organization is not enrolled. Everything you use individually is unaffected.",
  },
  capability_not_entitled: {
    zh: "当前团队的内测权限不包含该功能。个人使用的全部功能不受影响。",
    en: "This organization's beta access does not include this feature. Everything you use individually is unaffected.",
  },
  seat_limit_exceeded: {
    zh: "当前团队席位已满，你的账号不在生效席位内。请联系团队管理员调整席位。",
    en: "This organization is over its seat limit and your account is not within the active seats. Ask a team admin to adjust seats.",
  },

  // --- Access revoked ---
  // The desktop client wipes every locally cached team key whenever the control
  // plane answers 403, so these two must both say so. They differ in cause:
  // `forbidden` means the membership is gone; the entitlement codes above mean
  // the organization's access lapsed while the membership is intact.
  forbidden: {
    zh: "你的账号已不在该团队，或当前角色无权访问。本机缓存的团队密钥已清除。",
    en: "Your account is no longer in this organization, or your role lacks access. Locally cached team keys have been cleared.",
  },
  team_key_membership_revoked: {
    zh: "团队访问权限已失效，本机缓存的团队密钥已清除。",
    en: "Team access is no longer valid. Locally cached team keys have been cleared.",
  },

  // --- Control plane connectivity ---
  product_auth_control_plane_not_configured: {
    zh: "团队服务尚未就绪。团队功能目前为邀请制内测；若你在自建控制面，请设置环境变量 OPENSUNSTAR_CONTROL_PLANE_URL。",
    en: "Team service is not available. Team features are in invite-only beta; if you self-host a control plane, set the OPENSUNSTAR_CONTROL_PLANE_URL environment variable.",
  },
  product_auth_control_plane_url_invalid: {
    zh: "团队服务地址格式无效，请检查 URL 是否正确（示例：https://cp.example.com）。",
    en: "Team service URL is malformed. Check the URL format (e.g. https://cp.example.com).",
  },
  product_auth_control_plane_url_insecure: {
    zh: "团队服务地址必须使用 HTTPS（本机 127.0.0.1 开发除外）。",
    en: "Team service URL must use HTTPS (except localhost for development).",
  },
  product_auth_control_plane_unavailable: {
    zh: "无法连接到团队服务，请检查网络或服务是否在线。",
    en: "Cannot reach the team service. Check your network or whether the service is running.",
  },
  product_team_control_plane_unavailable: {
    zh: "无法连接到团队服务，请检查网络或服务是否在线。",
    en: "Cannot reach the team service. Check your network or whether the service is running.",
  },

  // --- Login flow ---
  product_auth_client_failed: {
    zh: "登录初始化失败，请重试。",
    en: "Login initialization failed. Please retry.",
  },
  product_auth_config_failed: {
    zh: "获取登录配置失败，团队服务可能暂时不可用。",
    en: "Failed to fetch login configuration. The team service may be temporarily unavailable.",
  },
  product_auth_config_invalid: {
    zh: "登录配置格式异常，请联系管理员检查团队服务部署。",
    en: "Login configuration is invalid. Ask your admin to check the team service deployment.",
  },
  product_auth_loopback_bind_failed: {
    zh: "无法启动本地登录回调端口，请关闭占用端口的程序后重试。",
    en: "Cannot bind the local login callback port. Close programs occupying the port and retry.",
  },
  product_auth_loopback_address_failed: {
    zh: "无法获取本地回调地址。",
    en: "Cannot determine the local callback address.",
  },
  product_auth_browser_open_failed: {
    zh: "无法打开系统浏览器完成登录，请手动复制登录链接。",
    en: "Cannot open the system browser. Copy the login link manually.",
  },
  product_auth_callback_timeout: {
    zh: "登录超时（180 秒），请重新点击登录按钮。",
    en: "Login timed out (180s). Click the login button again.",
  },
  product_auth_callback_failed: {
    zh: "登录回调处理失败，请重试。",
    en: "Login callback processing failed. Please retry.",
  },
  product_auth_login_cancelled: {
    zh: "登录已取消。",
    en: "Login cancelled.",
  },
  product_auth_callback_read_timeout: {
    zh: "等待登录回调超时，请重试。",
    en: "Timed out waiting for login callback. Please retry.",
  },
  product_auth_callback_read_failed: {
    zh: "读取登录回调失败。",
    en: "Failed to read the login callback.",
  },
  product_auth_callback_invalid: {
    zh: "登录回调数据无效，请重试。",
    en: "Login callback data is invalid. Please retry.",
  },
  product_auth_callback_response_failed: {
    zh: "登录回调响应失败。",
    en: "Login callback response failed.",
  },
  product_auth_callback_path_invalid: {
    zh: "登录回调路径无效。",
    en: "Login callback path is invalid.",
  },

  // --- Token exchange & session ---
  product_auth_exchange_unavailable: {
    zh: "登录验证服务不可用，请稍后重试。",
    en: "Login verification service is unavailable. Try again later.",
  },
  product_auth_exchange_failed: {
    zh: "登录验证失败，请重新登录。",
    en: "Login verification failed. Please log in again.",
  },
  product_auth_exchange_invalid: {
    zh: "登录验证返回数据异常。",
    en: "Login verification returned unexpected data.",
  },
  product_auth_expiry_invalid: {
    zh: "会话有效期数据异常，请重新登录。",
    en: "Session expiry data is invalid. Please log in again.",
  },
  product_auth_session_required: {
    zh: "请先登录 OpenSunstar 账户。",
    en: "Please log in to your OpenSunstar account first.",
  },
  product_auth_device_storage_failed: {
    zh: "设备信息安全存储失败，请检查系统密钥链权限。",
    en: "Failed to store device credentials. Check OS keychain permissions.",
  },

  // --- Token refresh ---
  product_auth_refresh_unavailable: {
    zh: "会话续期服务不可用，请重新登录。",
    en: "Session refresh service is unavailable. Please log in again.",
  },
  product_auth_refresh_failed: {
    zh: "会话已过期，请重新登录。",
    en: "Session expired. Please log in again.",
  },
  product_auth_refresh_invalid: {
    zh: "会话续期返回数据异常，请重新登录。",
    en: "Session refresh returned unexpected data. Please log in again.",
  },

  // --- Team operations ---
  product_team_organization_response_invalid: {
    zh: "组织操作返回数据异常，请重试。",
    en: "Organization operation returned unexpected data. Please retry.",
  },
  product_team_invite_response_invalid: {
    zh: "邀请操作返回数据异常，请重试。",
    en: "Invite operation returned unexpected data. Please retry.",
  },
  product_team_response_invalid: {
    zh: "团队服务返回数据异常，请重试。",
    en: "Team service returned unexpected data. Please retry.",
  },
  product_team_identifier_invalid: {
    zh: "组织标识格式无效（仅允许字母、数字、连字符和下划线）。",
    en: "Organization identifier is invalid (only letters, digits, hyphens, and underscores allowed).",
  },
};

/**
 * Reduce a raw error string to the bare code we can look up.
 *
 * Two wrappers have to come off:
 *  1. Tauri's invoke wrapper, added on the JS side of the bridge.
 *  2. `authenticated_json`'s HTTP wrapper, `product_team_request_failed_<status>`
 *     optionally followed by `:<code>` carrying the control plane's own code.
 *     Without unwrapping (2) every entitlement denial would render as the raw
 *     `product_team_request_failed_403`.
 */
function normalizeErrorCode(raw: string): string {
  const code = raw
    .replace(/^Error invoking remote command '.*?':\s*/, "")
    .trim();

  const httpWrapper = /^product_team_request_failed_\d{3}(?::(.+))?$/.exec(
    code,
  );
  if (httpWrapper) return httpWrapper[1]?.trim() || code;

  return code;
}

/**
 * Translate a raw error code/message from Rust into a user-friendly string.
 * Falls back to the original string if no mapping exists.
 */
export function translateProductError(raw: string, locale?: string): string {
  const code = normalizeErrorCode(raw);
  const lang = locale ?? navigator.language;
  const zh = lang.startsWith("zh");

  const entry = ERROR_MESSAGES[code];
  if (entry) return zh ? entry.zh : entry.en;

  // An unwrapped HTTP failure means the control plane answered without an error
  // code. Keep the status visible for support, but never show the bare
  // `product_team_request_failed_500` token to the user.
  const bareStatus = /^product_team_request_failed_(\d{3})$/.exec(code);
  if (bareStatus) {
    return zh
      ? `团队服务请求失败（HTTP ${bareStatus[1]}），请稍后重试。`
      : `Team service request failed (HTTP ${bareStatus[1]}). Please try again later.`;
  }

  return code;
}

/**
 * Check if a raw error string is a known product auth error code.
 */
export function isKnownProductError(raw: string): boolean {
  return normalizeErrorCode(raw) in ERROR_MESSAGES;
}
