/**
 * ClipHistory — 设置面板模块
 */

/**
 * 打开设置面板
 */
async function openSettings() {
  const overlay = document.getElementById('settingsOverlay');
  const panel = document.getElementById('settingsPanel');
  if (!overlay || !panel) return;

  // 加载当前设置
  await loadSettingsIntoForm();
  // 加载存储统计
  await loadStorageStats();

  overlay.classList.add('settings-overlay--visible');
  panel.classList.add('settings-panel--open');
}

/**
 * 关闭设置面板
 */
function closeSettings() {
  const overlay = document.getElementById('settingsOverlay');
  const panel = document.getElementById('settingsPanel');
  if (overlay) overlay.classList.remove('settings-overlay--visible');
  if (panel) panel.classList.remove('settings-panel--open');
}

/**
 * 从后端加载设置并填充表单
 */
async function loadSettingsIntoForm() {
  try {
    const settings = await window.__TAURI_INVOKE__('get_settings');
    document.getElementById('retentionDays').value = settings.retention_days;
    document.getElementById('storageCap').value = settings.storage_cap_mb;
    document.getElementById('autostartToggle').checked = settings.autostart;
  } catch (err) {
    console.error('加载设置失败:', err);
  }
}

/**
 * 从后端加载存储统计
 */
async function loadStorageStats() {
  try {
    const stats = await window.__TAURI_INVOKE__('get_storage_stats');
    const capMb = parseInt(document.getElementById('storageCap').value) || 500;
    const usagePercent = capMb > 0 ? (stats.total_bytes / (capMb * 1024 * 1024)) * 100 : 0;

    document.getElementById('statItemCount').textContent = stats.item_count;
    document.getElementById('statTotalSize').textContent = formatBytes(stats.total_bytes);
    document.getElementById('statUsageBar').style.width = Math.min(usagePercent, 100) + '%';
  } catch (err) {
    console.error('加载存储统计失败:', err);
  }
}

/**
 * 保存设置
 */
async function saveSettings() {
  const settings = {
    retention_days: parseInt(document.getElementById('retentionDays').value) || 30,
    storage_cap_mb: parseInt(document.getElementById('storageCap').value) || 500,
    autostart: document.getElementById('autostartToggle').checked,
  };

  console.log('保存设置:', settings);

  try {
    const fn = window.__TAURI_INVOKE__;
    console.log('invoke 函数可用:', typeof fn);
    if (!fn) {
      alert('内部错误: invoke 不可用');
      return;
    }
    await fn('update_settings', { settings });
    console.log('设置已保存，关闭面板');
    closeSettings();
    // 刷新历史（可能触发清理）
    if (typeof loadHistory === 'function') {
      await loadHistory();
    }
  } catch (err) {
    console.error('保存设置失败:', err);
    alert('保存失败: ' + err);
  }
}

/**
 * 文件大小格式化
 */
function formatBytes(bytes) {
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
  return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
}

// 暴露到全局
window.openSettings = openSettings;
window.closeSettings = closeSettings;
window.saveSettings = saveSettings;
