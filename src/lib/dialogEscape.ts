/**
 * 嵌套弹层的键盘事件归属判定。
 *
 * 背景：抽屉/浮层为了自己实现 Esc 关闭与 Tab 焦点环，会在 `document` 上挂全局
 * keydown 监听。但它内部还会再弹出对话框（例如「修复配置漂移」这种会覆盖用户
 * 手改内容的高危确认框）。此时一次 Esc 会被两层同时消费：子对话框关掉了，
 * 外层抽屉也一起关掉了 —— 用户只想撤销这次修复，结果连上下文都没了。
 *
 * 层叠（z-index）解决的是「谁盖在谁上面」，键盘事件的归属得另外判。
 */

/**
 * Radix `DialogContent` 渲染 `role="dialog"` + `data-state="open"`
 * （`@radix-ui/react-dialog@1.1.16`）。注意外层抽屉的外壳虽然也标了
 * `role="dialog"`，但没有 `data-state`，因此不会被这个选择器命中。
 */
const NESTED_MODAL_SELECTOR =
  '[role="dialog"][data-state="open"],[role="alertdialog"][data-state="open"]';

/**
 * 判断这次键盘事件是否已经归属于「更内层的弹层」，外层容器应当放手不管。
 *
 * 两个彼此独立的信号，任一命中即算归属内层：
 *
 * 1. `event.defaultPrevented` —— Radix 的 DismissableLayer 在 **document 的捕获
 *    阶段**处理 Escape，确认自己是最上层后先 `event.preventDefault()` 再
 *    `onDismiss()`（`@radix-ui/react-dismissable-layer@1.1.12`
 *    `dist/index.js:97-105`）。外层容器的监听器挂在 document 的**冒泡阶段**，
 *    必然晚于捕获阶段，所以这里读到的 `defaultPrevented` 一定已经反映了内层
 *    的处理结果。Tab 同理：Radix FocusScope 在边界上也会 `preventDefault()`。
 *
 * 2. DOM 探测 —— 万一内层不是 Radix，或将来换了实现不再 `preventDefault()`，
 *    再兜一层：文档里是否还存在处于 open 状态的模态对话框。
 *
 * @param event 原生键盘事件（只用到 `defaultPrevented`，便于单测构造）
 * @param container 外层容器的 DOM 节点。**包含** container 的对话框是它的
 *   祖先弹层而非子弹层，不参与判定；不包含它的（含 portal 到 `document.body`
 *   的兄弟节点）才算内层。
 */
export function isKeyEventOwnedByNestedLayer(
  event: Pick<KeyboardEvent, "defaultPrevented">,
  container: HTMLElement | null,
): boolean {
  if (event.defaultPrevented) return true;

  const doc =
    container?.ownerDocument ??
    (typeof document === "undefined" ? null : document);
  if (!doc) return false;

  const layers = doc.querySelectorAll<HTMLElement>(NESTED_MODAL_SELECTOR);
  return Array.from(layers).some(
    (layer) => layer !== container && !layer.contains(container),
  );
}
