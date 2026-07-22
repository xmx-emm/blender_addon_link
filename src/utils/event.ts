const RIGHT_CLICK_GUARD_KEY = '__blender_link_disable_right_click';

/** 禁止网页右键菜单（桌面工具常见做法） */
export function disableRightClick() {
  const g = globalThis as Record<string, unknown>;
  if (g[RIGHT_CLICK_GUARD_KEY]) return;
  g[RIGHT_CLICK_GUARD_KEY] = true;
  document.addEventListener('contextmenu', (event) => {
    event.preventDefault();
  });
}
