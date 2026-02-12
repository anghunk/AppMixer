<template>
  <div class="min-h-screen bg-white dark:bg-gray-950 text-gray-900 dark:text-gray-100 p-6">
    <div class="max-w-3xl mx-auto">
      <div class="flex justify-between items-center mb-8">
        <h1 class="text-xl font-bold flex items-center gap-3">
          AppMixer - 音量控制台
        </h1>
      </div>

      <!-- 系统主音量控制 -->
      <div class="mb-8 p-5 bg-blue-50 dark:bg-blue-900/20 border border-blue-100 dark:border-blue-800 rounded-xl shadow-sm">
        <div class="flex items-center gap-4">
          <div class="w-1/3 min-w-[150px] text-lg flex items-center gap-2">
            <span>💻 系统总音量</span>
          </div>
          <div class="flex-1 flex items-center gap-4">
            <div class="relative flex-1 h-8 flex items-center">
              <input 
                type="range" 
                min="0" 
                max="1" 
                step="0.01" 
                :value="systemVolume" 
                @input="updateSystemVolume($event)"
                class="w-full h-2 bg-gray-300 dark:bg-gray-600 rounded-lg appearance-none cursor-pointer accent-blue-600 hover:accent-blue-500"
                :disabled="systemMuted"
              />
            </div>
            <span class="w-12 text-right font-medium tabular-nums">
              {{ Math.round(systemVolume * 100) }}%
            </span>
          </div>
          <button 
            @click="toggleSystemMute"
            class="p-3 rounded-lg transition-colors duration-200 flex items-center justify-center w-12 h-12"
            :class="systemMuted ? 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400' : 'bg-gray-200 text-gray-600 dark:bg-gray-800 dark:text-gray-400 hover:bg-gray-300 dark:hover:bg-gray-700'"
            title="系统静音/取消静音"
          >
            <span v-if="systemMuted">🔇</span>
            <span v-else>🔊</span>
          </button>
        </div>
      </div>
      
      <div class="border-t border-gray-200 dark:border-gray-800 my-6"></div>
      
      <div v-if="sessions.length === 0" class="text-center py-12 text-gray-500">
        <p>没有检测到活动的音频应用</p>
        <button @click="fetchSessions" class="mt-4 px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
          刷新列表
        </button>
      </div>

      <div v-else class="space-y-4">
        <div 
          v-for="session in sessions" 
          :key="session.pid" 
          class="flex items-center gap-4 p-5 bg-gray-50 dark:bg-gray-900/50 border border-gray-200 dark:border-gray-800 rounded-xl shadow-sm transition-all hover:shadow-md"
        >
          <!-- 图标/名称 -->
          <div class="w-1/3 min-w-[150px] flex items-center gap-3">
             <div class="w-8 h-8 flex-shrink-0 bg-gray-200 dark:bg-gray-700 rounded-lg overflow-hidden flex items-center justify-center">
               <img v-if="session.icon" :src="'data:image/png;base64,' + session.icon" class="w-full h-full object-contain" />
               <span v-else class="text-xl">🎵</span>
             </div>
             <div class="overflow-hidden">
               <div class="fotruncate text-lg" :title="session.name">{{ session.name }}</div>
               <div class="text-xs text-gray-500 font-mono">PID: {{ session.pid }}</div>
             </div>
          </div>

          <!-- 音量滑块 -->
          <div class="flex-1 flex items-center gap-4">
            <div class="relative flex-1 h-8 flex items-center">
              <input 
                type="range" 
                min="0" 
                max="1" 
                step="0.01" 
                :value="session.volume" 
                @input="updateVolume(session.pid, $event)"
                class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-600 hover:accent-blue-500"
                :disabled="session.muted"
              />
            </div>
            <span class="w-12 text-right font-medium tabular-nums">
              {{ Math.round(session.volume * 100) }}%
            </span>
          </div>

          <!-- 静音按钮 -->
          <button 
            @click="toggleMute(session)"
            class="p-3 rounded-lg transition-colors duration-200 flex items-center justify-center w-12 h-12"
            :class="session.muted ? 'bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400' : 'bg-gray-200 text-gray-600 dark:bg-gray-800 dark:text-gray-400 hover:bg-gray-300 dark:hover:bg-gray-700'"
            title="静音/取消静音"
          >
            <span v-if="session.muted">🔇</span>
            <span v-else>🔊</span>
          </button>
        </div>
      </div>
      
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

interface AudioSession {
  pid: number;
  name: string;
  volume: number;
  muted: boolean;
  icon?: string;
}

const sessions = ref<AudioSession[]>([]);
const systemVolume = ref(0.5);
const systemMuted = ref(false);
const isDark = ref(false);
let intervalId: number | null = null;
let isDragging = false;
let isDraggingSystem = false;

async function fetchSessions() {
  if (isDragging) return;

  try {
    const newSessions = await invoke<AudioSession[]>('get_audio_sessions');
    
    // 简单的合并策略：如果列表长度不同或 PID 不同，直接替换
    // 如果完全一样，检查值是否有变化
    if (JSON.stringify(newSessions) !== JSON.stringify(sessions.value)) {
       sessions.value = newSessions;
    }
    
    // Also fetch system volume if not dragging
    if (!isDraggingSystem) {
      await fetchSystemVolume();
    }
  } catch (e) {
    console.error('Failed to fetch sessions:', e);
  }
}

async function fetchSystemVolume() {
  try {
    systemVolume.value = await invoke<number>('get_system_volume');
    systemMuted.value = await invoke<boolean>('get_system_mute');
  } catch (e) {
    console.error('Failed to fetch system volume:', e);
  }
}

function updateVolume(pid: number, event: Event) {
  isDragging = true;
  const input = event.target as HTMLInputElement;
  const volume = parseFloat(input.value);
  
  // 乐观更新
  const session = sessions.value.find(s => s.pid === pid);
  if (session) session.volume = volume;

  invoke('set_volume', { pid, volume }).catch(e => {
    console.error('Failed to set volume:', e);
  });
  
  resetDragging();
}

function updateSystemVolume(event: Event) {
  isDraggingSystem = true;
  const input = event.target as HTMLInputElement;
  const volume = parseFloat(input.value);
  
  systemVolume.value = volume;

  invoke('set_system_volume', { volume }).catch(e => {
    console.error('Failed to set system volume:', e);
  });
  
  resetDraggingSystem();
}

let dragTimeout: number | null = null;
function resetDragging() {
  if (dragTimeout) clearTimeout(dragTimeout);
  dragTimeout = window.setTimeout(() => {
    isDragging = false;
  }, 1000);
}

let dragSystemTimeout: number | null = null;
function resetDraggingSystem() {
  if (dragSystemTimeout) clearTimeout(dragSystemTimeout);
  dragSystemTimeout = window.setTimeout(() => {
    isDraggingSystem = false;
  }, 1000);
}

async function toggleMute(session: AudioSession) {
  const newMuted = !session.muted;
  session.muted = newMuted;

  try {
    await invoke('set_mute', { pid: session.pid, muted: newMuted });
  } catch (e) {
    console.error('Failed to toggle mute:', e);
    session.muted = !newMuted; // Revert
  }
}

async function toggleSystemMute() {
  const newMuted = !systemMuted.value;
  systemMuted.value = newMuted;

  try {
    await invoke('set_system_mute', { muted: newMuted });
  } catch (e) {
    console.error('Failed to toggle system mute:', e);
    systemMuted.value = !newMuted; // Revert
  }
}

onMounted(() => {
  if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
    isDark.value = true;
    document.documentElement.classList.add("dark");
  }

  fetchSessions();
  intervalId = window.setInterval(fetchSessions, 2000);
});

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId);
});
</script>
