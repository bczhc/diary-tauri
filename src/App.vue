<template>
  <div class="app-wrapper">
    <transition name="fade" mode="out-in">

      <div v-if="!isUnlocked" key="login" class="auth-container">
        <div class="auth-card">
          <div class="icon">🌿</div>
          <h1>个人日记存根</h1>
          <p class="subtitle">请输入访问密钥以解密数据库</p>
          <div class="input-group">
            <input
                v-model="password"
                type="password"
                placeholder="Enter Key..."
                v-on:keydown="handleKeydown"
                :class="{ 'input-error': error }"
            />
            <button v-on:click="unlockDatabase" :disabled="loading">
              {{ loading ? '正在解密...' : '解锁' }}
            </button>
          </div>
          <p v-if="error" class="error-msg">{{ error }}</p>
        </div>
      </div>

      <div v-else key="content" class="main-content">
        <aside class="sidebar">
          <div class="sidebar-header">
            <div class="title-area">
              <h2>历程</h2>
              <span class="count">{{ dateList.length }} 篇</span>
            </div>
            <div class="search-bar">
              <input
                  v-model="searchQuery"
                  type="text"
                  placeholder="搜索日期或内容..."
                  v-on:input="handleSearch"
              />
            </div>
          </div>

          <div class="date-list">
            <div
                v-for="date in dateList"
                :key="date"
                class="date-card"
                v-on:click="handleDateClick(date)"
                :class="{ 'active-card': selectedDate === date }"
            >
              <div class="calendar-box">
                <span class="day-num">{{ date.toString().substring(6, 8) }}</span>
              </div>
              <div class="date-info">
                <span class="date-text">{{ formatDate(date) }}</span>
                <span class="weekday-text">{{ getDayOfWeek(date) }}</span>
              </div>
            </div>

            <div v-if="dateList.length === 0" class="no-results">
              没有找到匹配的日记
            </div>
          </div>
        </aside>

        <main class="viewer">
          <div v-if="selectedDate" class="editor-container">
            <div class="editor-header">
              <div class="header-main">
                <h3>{{ formatDate(selectedDate) }} · {{ getDayOfWeek(selectedDate) }}</h3>
                <div class="stats-line">
                  <span class="word-count">共 {{ displayWordCount }} 字</span>
                  <span v-if="saveStatus" class="save-indicator" style="visibility: hidden">{{ saveStatus }}</span>
                </div>
              </div>

              <div class="header-controls">
                <div class="font-control">
                  <button v-on:click="adjustFontSize(-2)" title="减小字号">A-</button>
                  <span class="font-size-label">{{ fontSize }}px</span>
                  <button v-on:click="adjustFontSize(2)" title="增大字号">A+</button>
                </div>

                <div
                    class="mode-toggle"
                    v-on:click="toggleEditMode"
                    :class="{ 'is-editing-mode': isEditing }"
                >
                  <span class="mode-badge">{{ isEditing ? '编辑模式' : '预览模式' }}</span>
                </div>
              </div>
            </div>
            <textarea
                v-model="currentContent"
                :readonly="!isEditing"
                class="diary-textarea"
                :style="{ fontSize: fontSize + 'px' }"
                :placeholder="isEditing ? '开始记录今天的生活...' : '无内容'"
            ></textarea>
          </div>
          <div v-else class="empty-state">
            <div class="empty-icon">📖</div>
            <p>已解锁，请选择日期</p>
          </div>
        </main>
      </div>

    </transition>
  </div>
</template>

<script setup>
import { ref, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const password = ref('');
const searchQuery = ref('');
const isUnlocked = ref(false);
const loading = ref(false);
const error = ref('');
const dateList = ref([]);
const selectedDate = ref(null);
const currentContent = ref('');

const fontSize = ref(16);

const isEditing = ref(false);
const saveStatus = ref('');
const lastSavedContent = ref('');
let autoSaveTimer = null;

const displayWordCount = ref(0);
let wordCountTimeout = null;

watch(currentContent, (newVal) => {
  if (wordCountTimeout) clearTimeout(wordCountTimeout);
  wordCountTimeout = setTimeout(() => {
    if (!newVal || newVal === '正在读取...') {
      displayWordCount.value = 0;
    } else {
      displayWordCount.value = Array.from(newVal).length;
    }
  }, 300);
}, { immediate: true });

const adjustFontSize = (delta) => {
  const next = fontSize.value + delta;
  if (next >= 12 && next <= 32) {
    fontSize.value = next;
  }
};

const handleKeydown = (event) => {
  if (event.key === 'Enter') {
    unlockDatabase();
  }
};

let searchTimeout = null;
const handleSearch = () => {
  if (searchTimeout) clearTimeout(searchTimeout);
  searchTimeout = setTimeout(async () => {
    try {
      const dates = await invoke('search_diary', {
        queryStr: searchQuery.value
      });
      dateList.value = dates;
    } catch (err) {
      console.error("Search failed", err);
    }
  }, 300);
};

const unlockDatabase = async () => {
  if (!password.value || loading.value) return;
  loading.value = true;
  error.value = '';

  try {
    const dates = await invoke('open_and_list_dates', { password: password.value });
    dateList.value = dates;
    isUnlocked.value = true;
  } catch (err) {
    error.value = typeof err === 'string' ? err : '解锁失败，请检查密钥';
    password.value = '';
  } finally {
    loading.value = false;
  }
};

const saveDiary = async () => {
  if (!selectedDate.value || currentContent.value === lastSavedContent.value) return;

  saveStatus.value = '正在保存...';
  try {
    await invoke('save_diary_content', {
      date: selectedDate.value,
      content: currentContent.value
    });
    lastSavedContent.value = currentContent.value;
    saveStatus.value = '已保存';
    setTimeout(() => {
      if (saveStatus.value === '已保存') saveStatus.value = '';
    }, 2000);
  } catch (err) {
    saveStatus.value = '保存失败';
    console.error(err);
  }
};

const toggleEditMode = async () => {
  if (isEditing.value) {
    await saveDiary();
    stopAutoSave();
    isEditing.value = false;
  } else {
    isEditing.value = true;
    startAutoSave();
  }
};

const startAutoSave = () => {
  stopAutoSave();
  autoSaveTimer = setInterval(() => {
    if (isEditing.value) saveDiary();
  }, 10000);
};

const stopAutoSave = () => {
  if (autoSaveTimer) {
    clearInterval(autoSaveTimer);
    autoSaveTimer = null;
  }
};

const handleDateClick = async (date) => {
  if (selectedDate.value === date) return;

  if (isEditing.value) {
    await saveDiary();
    stopAutoSave();
    isEditing.value = false;
  }

  await loadDiaryContent(date);
};

const loadDiaryContent = async (date) => {
  selectedDate.value = date;
  currentContent.value = '正在读取...';

  try {
    const content = await invoke('get_diary_content', { date: date });
    currentContent.value = content;
    lastSavedContent.value = content;
  } catch (err) {
    currentContent.value = '读取失败: ' + err;
  }
};

const formatDate = (dateInt) => {
  const s = dateInt.toString();
  if (s.length !== 8) return s;
  return `${s.substring(0, 4)}年${s.substring(4, 6)}月${s.substring(6, 8)}日`;
};

const getDayOfWeek = (dateInt) => {
  const s = dateInt.toString();
  if (s.length !== 8) return '';
  const dateObj = new Date(`${s.substring(0, 4)}-${s.substring(4, 6)}-${s.substring(6, 8)}`);
  return ['周日', '周一', '周二', '周三', '周四', '周五', '周六'][dateObj.getDay()];
};

onUnmounted(() => {
  stopAutoSave();
});
</script>

<style>
:root {
  --bg-color: #f9f7f2;
  --sidebar-bg: #ffffff;
  --primary-color: #5d6d7e;
  --accent-color: #aeb6bf;
  --text-main: #2c3e50;
  --text-sub: #95a5a6;
  --input-bg: #f5f6f7;
  --border-color: #edebe9;
  --edit-theme: #3498db;
}

body {
  margin: 0;
  background-color: var(--bg-color);
  color: var(--text-main);
  font-family: "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
  overflow: hidden;
}

.app-wrapper { height: 100vh; width: 100vw; }

.auth-container {
  height: 100vh; display: flex; align-items: center; justify-content: center;
  background: linear-gradient(135deg, #fdfbfb 0%, #ebedee 100%);
}
.auth-card {
  background: white; padding: 40px; border-radius: 24px;
  box-shadow: 0 20px 50px rgba(0,0,0,0.05); text-align: center; width: 350px;
}
.icon { font-size: 40px; margin-bottom: 10px; }
h1 { font-size: 20px; letter-spacing: 2px; margin: 10px 0; }
.subtitle { color: var(--text-sub); font-size: 13px; margin-bottom: 30px; }

.input-group { display: flex; flex-direction: column; gap: 12px; }
input {
  padding: 12px; border: 1px solid #eee; border-radius: 10px;
  background: var(--input-bg); font-size: 15px; outline: none; transition: all 0.3s;
}
input:focus { border-color: var(--primary-color); background: white; box-shadow: 0 0 0 3px rgba(93,109,126,0.08); }

button {
  padding: 14px; border: none; border-radius: 12px;
  background: var(--primary-color); color: white; cursor: pointer;
  font-weight: 600;
}

.main-content { display: flex; height: 100vh; }

.sidebar {
  width: 320px; background: var(--sidebar-bg); border-right: 1px solid var(--border-color);
  display: flex; flex-direction: column; flex-shrink: 0;
}
.sidebar-header { padding: 24px 20px 16px; display: flex; flex-direction: column; gap: 16px; }
.title-area { display: flex; justify-content: space-between; align-items: baseline; }
.title-area h2 { margin: 0; font-size: 20px; }
.count { font-size: 12px; color: var(--text-sub); }

.search-bar input {
  width: 100%; box-sizing: border-box; padding: 10px 14px; font-size: 13px;
  border-radius: 8px; border: 1px solid transparent;
}

.date-list { flex: 1; overflow-y: auto; padding: 0 12px 20px; }
.date-card {
  display: flex; align-items: center; padding: 12px; margin-bottom: 8px;
  border-radius: 14px; cursor: pointer; transition: all 0.2s;
  border: 1px solid transparent;
}
.date-card:hover { background: #f8f9fa; }
.active-card { background: #f0f3f6 !important; border-color: #e5e9ef !important; }

.calendar-box {
  width: 40px; height: 40px; background: #fff; border: 1px solid #eee;
  border-radius: 10px; display: flex; align-items: center; justify-content: center;
  margin-right: 14px;
}
.day-num { font-size: 16px; font-weight: bold; color: var(--primary-color); }
.date-info { display: flex; flex-direction: column; }
.date-text { font-size: 13.5px; font-weight: 500; }
.weekday-text { font-size: 11px; color: var(--text-sub); }

.viewer { flex: 1; display: flex; flex-direction: column; background: var(--bg-color); }
.editor-container {
  display: flex; flex-direction: column; height: 100%; padding: 40px;
  box-sizing: border-box; max-width: 900px; margin: 0 auto; width: 100%;
}
.editor-header {
  display: flex; justify-content: space-between; align-items: flex-end;
  margin-bottom: 24px; padding-bottom: 12px; border-bottom: 1px solid #e2e2e2;
}
.header-main { display: flex; flex-direction: column; gap: 6px; }
.header-main h3 { margin: 0; font-size: 18px; color: var(--primary-color); }

.stats-line { display: flex; align-items: center; gap: 12px; }
.word-count { font-size: 12px; color: var(--text-sub); }
.save-indicator { font-size: 12px; color: var(--accent-color); font-style: italic; }

.header-controls { display: flex; align-items: center; gap: 20px; }

.font-control {
  display: flex; align-items: center; background: #eee; border-radius: 8px; overflow: hidden;
}
.font-control button {
  padding: 4px 8px; border-radius: 0; background: transparent; color: #666; font-size: 11px;
}
.font-control button:hover { background: #e0e0e0; }
.font-size-label { font-size: 11px; width: 34px; text-align: center; color: #888; border-left: 1px solid #ddd; border-right: 1px solid #ddd; }

.mode-toggle {
  display: flex; align-items: center; cursor: pointer; user-select: none;
}
.mode-badge {
  font-size: 11px; color: var(--text-sub); border: 1px solid #ddd;
  padding: 4px 12px; border-radius: 20px; transition: all 0.2s;
}
.mode-toggle:hover .mode-badge { background: #eee; }
.is-editing-mode .mode-badge {
  background: var(--edit-theme); color: white; border-color: var(--edit-theme);
}

.diary-textarea {
  flex: 1; border: none; background: transparent; resize: none;
  line-height: 1.8; color: #444; outline: none;
  font-family: inherit; width: 100%; transition: background 0.3s, font-size 0.2s;
}
.diary-textarea:not([readonly]) {
  background: rgba(255,255,255,0.4);
  padding: 10px;
  border-radius: 8px;
}

.empty-state { flex: 1; display: flex; flex-direction: column; align-items: center; justify-content: center; color: var(--text-sub); }
.empty-icon { font-size: 48px; margin-bottom: 20px; opacity: 0.2; }

.fade-enter-active, .fade-leave-active { transition: all 0.3s ease; }
.fade-enter-from { opacity: 0; transform: translateY(5px); }
.fade-leave-to { opacity: 0; transform: translateY(-5px); }
.error-msg { color: #e74c3c; font-size: 12px; margin-top: 15px; }
</style>