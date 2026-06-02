/**
 * ClipHistory — 卡片列表渲染模块
 * 负责渲染、更新、搜索过滤历史记录卡片
 */

// 当前显示的数据
let currentClips = [];
let lastDeletedId = null;
let undoTimer = null;

/**
 * 获取格式化的相对时间
 */
function formatTime(timestamp) {
  const now = Math.floor(Date.now() / 1000);
  const diff = now - timestamp;

  if (diff < 30) return '刚刚';
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
  if (diff < 604800) return `${Math.floor(diff / 86400)} 天前`;

  const date = new Date(timestamp * 1000);
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
}

/**
 * 渲染单张卡片 HTML
 */
function renderCard(item) {
  const pinnedClass = item.is_pinned ? ' card--pinned' : '';
  const pinIcon = item.is_pinned ? '📌' : '📌';

  let bodyHtml = '';
  if (item.content_type === 'text') {
    const text = item.text_content || '';
    bodyHtml = `<div class="card__text">${escapeHtml(text)}</div>`;
  } else if (item.content_type === 'image') {
    const src = item._dataUrl || '';
    bodyHtml = `<img class="card__image" src="${src}" alt="剪贴板图片" loading="lazy">`;
  }

  return `
    <div class="card${pinnedClass}" data-id="${item.id}" data-type="${item.content_type}">
      <div class="card__content">${bodyHtml}</div>
      <div class="card__footer">
        <span class="card__time">${formatTime(item.created_at)}</span>
        <div class="card__actions">
          <button class="card__pin-icon" title="${item.is_pinned ? '取消置顶' : '置顶'}">${pinIcon}</button>
          <button class="card__delete-btn" title="删除">✕</button>
        </div>
      </div>
    </div>`;
}

/**
 * 渲染全部卡片到 DOM
 */
function renderCardList(clips) {
  const container = document.getElementById('cardList');
  const placeholder = document.getElementById('placeholder');

  if (!clips || clips.length === 0) {
    container.innerHTML = '';
    if (placeholder) placeholder.style.display = 'flex';
    return;
  }

  if (placeholder) placeholder.style.display = 'none';
  container.innerHTML = clips.map(renderCard).join('');
  bindCardEvents();
}

/**
 * 绑定卡片上的事件（事件委托）
 */
function bindCardEvents() {
  const container = document.getElementById('cardList');

  container.addEventListener('click', async (e) => {
    const card = e.target.closest('.card');
    if (!card) return;

    const id = parseInt(card.dataset.id);

    // 点击置顶按钮
    if (e.target.closest('.card__pin-icon')) {
      e.stopPropagation();
      try {
        const newState = await window.__TAURI_INVOKE__('toggle_pin', { id });
        const idx = currentClips.findIndex(c => c.id === id);
        if (idx !== -1) {
          currentClips[idx].is_pinned = newState;
        }
        refreshDisplay();
      } catch (err) {
        console.error('置顶操作失败:', err);
      }
      return;
    }

    // 点击删除按钮
    if (e.target.closest('.card__delete-btn')) {
      e.stopPropagation();
      deleteItem(id);
      return;
    }

    // 点击卡片主体 → 恢复到剪贴板
    try {
      await window.__TAURI_INVOKE__('restore_to_clipboard', { id });
      card.classList.add('card--copied');
      setTimeout(() => card.classList.remove('card--copied'), 500);
    } catch (err) {
      console.error('回贴失败:', err);
    }
  });
}

/**
 * 删除条目（带撤销）
 */
async function deleteItem(id) {
  const item = currentClips.find(c => c.id === id);
  if (!item) return;

  // 先缓存信息用于撤销
  const backup = { ...item };

  try {
    await window.__TAURI_INVOKE__('delete_item', { id });
  } catch (err) {
    console.error('删除失败:', err);
    return;
  }

  // 从当前列表移除
  currentClips = currentClips.filter(c => c.id !== id);
  refreshDisplay();

  // 显示撤销提示
  showUndoToast(id, backup);
}

/**
 * 撤销删除提示
 */
function showUndoToast(id, backup) {
  // 清除之前的定时器
  if (undoTimer) clearTimeout(undoTimer);

  // 移除之前的 toast
  const oldToast = document.getElementById('undoToast');
  if (oldToast) oldToast.remove();

  const toast = document.createElement('div');
  toast.id = 'undoToast';
  toast.className = 'undo-toast';
  toast.innerHTML = '<span>已删除</span><button class="undo-toast__link">撤销</button>';

  toast.querySelector('.undo-toast__link').addEventListener('click', async () => {
    // 撤销：重新插入数据库（通过调用空操作触发再刷新即可，简化处理——直接刷新列表）
    // 注意：这里用简单方式——向 main.js 暴露恢复函数
    if (window.__undoDelete) {
      await window.__undoDelete(backup);
    }
    toast.remove();
    clearTimeout(undoTimer);
  });

  document.body.appendChild(toast);

  undoTimer = setTimeout(() => {
    toast.classList.add('undo-toast--hiding');
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

/**
 * 根据搜索词过滤并重新渲染
 */
function filterCards(query) {
  if (!query || query.trim() === '') {
    renderCardList(currentClips);
    return;
  }

  const q = query.toLowerCase();
  const filtered = currentClips.filter(c => {
    if (c.content_type === 'text' && c.text_content) {
      return c.text_content.toLowerCase().includes(q);
    }
    return false; // 图片不可搜索
  });
  renderCardList(filtered);
}

/**
 * 从后端刷新并重新渲染
 */
function refreshDisplay() {
  // 保持排序：置顶优先，时间降序
  currentClips.sort((a, b) => {
    if (a.is_pinned !== b.is_pinned) return b.is_pinned - a.is_pinned;
    return b.created_at - a.created_at;
  });

  const searchInput = document.getElementById('searchInput');
  if (searchInput && searchInput.value.trim()) {
    filterCards(searchInput.value);
  } else {
    renderCardList(currentClips);
  }
}

/**
 * HTML 转义
 */
function escapeHtml(str) {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}
