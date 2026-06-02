/**
 * ClipHistory — 卡片列表渲染模块
 * 负责渲染、更新、搜索过滤历史记录卡片
 */

// 当前显示的数据
let currentClips = [];
let lastDeletedId = null;
let undoTimer = null;
let isPinning = false;       // 置顶操作锁，防止并发闪烁
let deletingIds = new Set(); // 正在删除中的 ID 集合，防止竞态重复
let pendingDeletes = [];     // 待撤销的删除项列表
let undoToastEl = null;      // 复用的撤销提示元素

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
}

/**
 * 绑定卡片上的事件（事件委托，整个生命周期只调用一次）
 */
function bindCardEvents() {
  const container = document.getElementById('cardList');
  if (!container) return;

  container.addEventListener('click', async (e) => {
    const card = e.target.closest('.card');
    if (!card) return;

    const id = parseInt(card.dataset.id);

    // 点击置顶按钮
    if (e.target.closest('.card__pin-icon')) {
      e.stopPropagation();
      if (isPinning) return;
      isPinning = true;

      try {
        const newState = await window.__TAURI_INVOKE__('toggle_pin', { id });
        const idx = currentClips.findIndex(c => c.id === id);
        if (idx !== -1) {
          currentClips[idx].is_pinned = newState;
        }
      } catch (err) {
        console.error('置顶操作失败:', err);
      } finally {
        isPinning = false;
        refreshDisplay(); // 无论成败都重新渲染，恢复按钮状态
      }
      return;
    }

    // 点击删除按钮
    if (e.target.closest('.card__delete-btn')) {
      e.stopPropagation();
      if (deletingIds.has(id)) return;
      deletingIds.add(id);
      try {
        await deleteItem(id);
      } finally {
        deletingIds.delete(id);
      }
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

  const backup = { ...item, _dataUrl: undefined }; // 不缓存 base64 数据

  try {
    await window.__TAURI_INVOKE__('delete_item', { id });
  } catch (err) {
    console.error('删除失败:', err);
    return;
  }

  // 从当前列表移除
  currentClips = currentClips.filter(c => c.id !== id);
  pendingDeletes.push({ id, backup });
  refreshDisplay();

  // 更新撤销提示（不销毁重建）
  showUndoToast();
}

/**
 * 撤销删除提示（复用元素，避免闪烁）
 */
function showUndoToast() {
  if (undoTimer) clearTimeout(undoTimer);

  // 复用已有toast或创建新的
  if (!undoToastEl || !document.body.contains(undoToastEl)) {
    undoToastEl = document.createElement('div');
    undoToastEl.id = 'undoToast';
    undoToastEl.className = 'undo-toast';
    undoToastEl.innerHTML = '<span id="undoToastText">已删除</span><button class="undo-toast__link" id="undoToastBtn">撤销</button>';
    document.body.appendChild(undoToastEl);

    // 撤销按钮事件（只绑定一次）
    document.getElementById('undoToastBtn').addEventListener('click', async () => {
      const items = [...pendingDeletes];
      if (items.length === 0) return; // toast 可见但数据已清空，忽略
      pendingDeletes = [];
      if (window.__undoDelete) {
        await window.__undoDelete(items);
      }
      undoToastEl.classList.add('undo-toast--hiding');
      setTimeout(() => {
        undoToastEl.classList.remove('undo-toast--hiding');
        undoToastEl.style.display = 'none';
      }, 300);
      clearTimeout(undoTimer);
    });
  }

  // 更新计数文案
  const count = pendingDeletes.length;
  document.getElementById('undoToastText').textContent =
    count > 1 ? `已删除 ${count} 条` : '已删除';

  // 确保可见
  undoToastEl.style.display = '';
  undoToastEl.classList.remove('undo-toast--hiding');

  // 3秒后自动消失（先动画，再清数据，防止竞态）
  undoTimer = setTimeout(() => {
    undoToastEl.classList.add('undo-toast--hiding');
    setTimeout(() => {
      pendingDeletes = [];
      undoToastEl.classList.remove('undo-toast--hiding');
      undoToastEl.style.display = 'none';
    }, 300);
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
    return false;
  });
  renderCardList(filtered);
}

/**
 * 从后端刷新并重新渲染
 */
function refreshDisplay() {
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
