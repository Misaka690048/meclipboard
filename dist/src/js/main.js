/**
 * ClipHistory — 主入口脚本
 * 通过 window.__TAURI__（withGlobalTauri: true）获取 Tauri API
 */

// Tauri webview 注入的全局 API（withGlobalTauri: true）
const tauriCore = window.__TAURI__?.core;
const tauriInternals = window.__TAURI_INTERNALS__;

console.log('ClipHistory 诊断:', {
  hasTauriGlobal: !!window.__TAURI__,
  hasTauriCore: !!tauriCore,
  hasTauriInternals: !!tauriInternals,
});

/**
 * invoke 封装（优先用 __TAURI__.core，fallback 到 __TAURI_INTERNALS__）
 */
async function invoke(cmd, args = {}) {
  if (tauriCore?.invoke) {
    return tauriCore.invoke(cmd, args);
  }
  if (tauriInternals?.invoke) {
    console.warn('使用 __TAURI_INTERNALS__.invoke fallback');
    return tauriInternals.invoke(cmd, args);
  }
  throw new Error('Tauri invoke API 不可用');
}

// 暴露给其他脚本使用
window.__TAURI_INVOKE__ = invoke;

// 记录上一次已知的最新 ID，用于检测新增条目
let lastKnownId = 0;

/**
 * 从后端加载完整历史记录
 */
async function loadHistory() {
  try {
    const clips = await invoke('get_history', { limit: 200 });
    currentClips = clips || [];
    if (currentClips.length > 0) {
      lastKnownId = Math.max(...currentClips.map(c => c.id));
    }
    // 先并行加载图片预览，再渲染
    await loadImagePreviews();
    refreshDisplay();
    console.log('历史记录加载完成，共', currentClips.length, '条');
  } catch (err) {
    console.error('加载历史记录失败:', err);
  }
}

/**
 * 轮询：检测新条目（每秒，取足够多防止遗漏）
 */
async function pollNewItems() {
  try {
    const clips = await invoke('get_history', { limit: 100 });
    if (!clips || clips.length === 0) return;

    const newClips = clips.filter(c => c.id > lastKnownId);
    if (newClips.length > 0) {
      console.log('检测到', newClips.length, '条新记录');
      lastKnownId = Math.max(...newClips.map(c => c.id), lastKnownId);

      for (const item of newClips.reverse()) {
        if (!currentClips.some(c => c.id === item.id)) {
          currentClips.unshift(item);
          if (item.content_type === 'image') {
            item._dataUrl = await invoke('get_image_data_url', { id: item.id }).catch(() => null);
          }
        }
      }
      refreshDisplay();
    }
  } catch (err) {
    console.warn('轮询失败:', err);
  }
}

/**
 * 为已有列表中的图片加载预览（并行化）
 */
async function loadImagePreviews() {
  const imageItems = currentClips.filter(item => item.content_type === 'image');
  if (imageItems.length === 0) return;
  await Promise.all(imageItems.map(async (item) => {
    try {
      item._dataUrl = await invoke('get_image_data_url', { id: item.id });
    } catch (err) { /* 静默处理 */ }
  }));
}

/**
 * 撤销删除：逐条重新插入数据库
 */
window.__undoDelete = async function(items) {
  const list = Array.isArray(items) ? items : [items];
  for (const entry of list) {
    const backup = entry.backup || entry;
    try {
      await invoke('restore_deleted_item', { item: backup });
    } catch (err) {
      console.error('撤销删除失败:', err);
    }
  }
  await loadHistory();
};

/**
 * 搜索输入（防抖 250ms）
 */
function setupSearch() {
  const input = document.getElementById('searchInput');
  if (!input) return;
  let timer;
  input.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(() => filterCards(input.value), 250);
  });
}

/**
 * 启动
 */
document.addEventListener('DOMContentLoaded', async () => {
  console.log('ClipHistory 启动 (window.__TAURI__ 模式)');
  setupSearch();
  bindCardEvents();

  const settingsBtn = document.getElementById('settingsBtn');
  if (settingsBtn) {
    settingsBtn.addEventListener('click', () => window.openSettings());
  }

  await loadHistory();
  setInterval(pollNewItems, 1000);
});
