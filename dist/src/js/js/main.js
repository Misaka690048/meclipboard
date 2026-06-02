/**
 * ClipHistory — 主入口脚本
 * 通过 window.__TAURI__（withGlobalTauri: true）获取 Tauri API
 * 不再从 node_modules 导入，避免 dev/prod 路径不一致问题
 */

// Tauri webview 注入的全局 API（withGlobalTauri: true）
const tauriCore = window.__TAURI__?.core;
if (!tauriCore) {
  console.error('Tauri API 不可用，请确认在 Tauri webview 中运行');
}

/**
 * invoke 封装
 */
async function invoke(cmd, args = {}) {
  if (!tauriCore) throw new Error('Tauri API 不可用');
  return tauriCore.invoke(cmd, args);
}

// 暴露给非模块脚本（history.js, settings.js）使用
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
    loadImagePreviews();
    refreshDisplay();
    console.log('历史记录加载完成，共', currentClips.length, '条');
  } catch (err) {
    console.error('加载历史记录失败:', err);
  }
}

/**
 * 轮询：检测新条目（每秒）
 */
async function pollNewItems() {
  try {
    const clips = await invoke('get_history', { limit: 20 });
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
    // 轮询静默处理
  }
}

/**
 * 为已有列表中的图片加载预览
 */
async function loadImagePreviews() {
  for (const item of currentClips) {
    if (item.content_type !== 'image') continue;
    try {
      item._dataUrl = await invoke('get_image_data_url', { id: item.id });
      const imgEl = document.querySelector(`.card[data-id="${item.id}"] .card__image`);
      if (imgEl) imgEl.src = item._dataUrl;
    } catch (err) { /* 静默处理 */ }
  }
}

/**
 * 撤销删除
 */
window.__undoDelete = async function(backup) {
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

  // 设置面板齿轮按钮
  const settingsBtn = document.getElementById('settingsBtn');
  if (settingsBtn) {
    settingsBtn.addEventListener('click', () => window.openSettings());
  }

  await loadHistory();
  setInterval(pollNewItems, 1000);
});
