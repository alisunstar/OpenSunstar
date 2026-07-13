export function validateChangeId(changeId: string): string | null {
  if (changeId.trim() !== changeId) {
    return "Change ID 不能包含首尾空白";
  }
  if (changeId.length < 3 || changeId.length > 80) {
    return "Change ID 长度必须在 3 到 80 个字符之间";
  }
  if (changeId === "." || changeId === "..") {
    return "Change ID 不能使用 . 或 ..";
  }
  if (!/^[A-Za-z0-9._-]+$/.test(changeId)) {
    return "Change ID 只能包含英文字母、数字、点、下划线和短横线";
  }
  return null;
}
