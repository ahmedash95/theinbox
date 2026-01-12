<script setup lang="ts">
import { computed } from "vue";
import { RefreshCw, RefreshCcw, Check, ChevronRight, Mail } from "lucide-vue-next";
import type { EmailWithMatches } from "../types";

const props = defineProps<{
  emails: EmailWithMatches[];
  loading: boolean;
  marking: boolean;
  markingCount: number;
  selectedIds: Set<string>;
}>();

const emit = defineEmits<{
  (e: "toggle-select", id: string): void;
  (e: "select-all"): void;
  (e: "deselect-all"): void;
  (e: "mark-read"): void;
  (e: "refresh"): void;
  (e: "force-refresh"): void;
}>();

const allSelected = computed(() => {
  return props.emails.length > 0 && props.selectedIds.size === props.emails.length;
});

const someSelected = computed(() => {
  return props.selectedIds.size > 0 && props.selectedIds.size < props.emails.length;
});

// Clean sender name - remove quotes and extract name from "Name <email>" format
function formatSender(sender: string): string {
  // Remove surrounding quotes if present
  let cleaned = sender.replace(/^["']|["']$/g, '').trim();
  
  // Extract name from "Name <email@example.com>" format
  const match = cleaned.match(/^(.+?)\s*<.+>$/);
  if (match) {
    cleaned = match[1].trim();
  }
  
  // Remove any remaining quotes
  cleaned = cleaned.replace(/^["']|["']$/g, '').trim();
  
  return cleaned || sender;
}

function formatDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const isToday = date.toDateString() === now.toDateString();
    
    if (isToday) {
      return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
    }
    
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);
    if (date.toDateString() === yesterday.toDateString()) {
      return "Yesterday";
    }
    
    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  } catch {
    return dateStr;
  }
}
</script>

<template>
  <div class="list-container">
    <!-- Progress Bar -->
    <div v-if="marking" class="progress-bar-container">
      <div class="progress-bar-text">
        Marking {{ markingCount }} {{ markingCount === 1 ? 'email' : 'emails' }} as read...
      </div>
      <div class="progress-bar">
        <div class="progress-bar-fill"></div>
      </div>
    </div>

    <!-- SwiftUI-style Toolbar -->
    <div class="toolbar">
      <div class="toolbar-leading">
        <label class="checkbox-wrapper" title="Select all">
          <input
            type="checkbox"
            :checked="allSelected"
            :indeterminate="someSelected"
            @change="allSelected ? emit('deselect-all') : emit('select-all')"
            :disabled="emails.length === 0 || loading"
          />
        </label>
        <span class="toolbar-count">{{ emails.length }} messages</span>
      </div>
      
      <div class="toolbar-trailing">
        <button
          class="toolbar-button"
          @click="emit('refresh')"
          :disabled="loading || marking"
          title="Refresh"
        >
          <RefreshCw :size="15" />
        </button>
        <button
          class="toolbar-button"
          @click="emit('force-refresh')"
          :disabled="loading || marking"
          title="Force refresh from Mail"
        >
          <RefreshCcw :size="15" />
        </button>
        <div class="toolbar-divider"></div>
        <button
          class="toolbar-button primary"
          @click="emit('mark-read')"
          :disabled="selectedIds.size === 0 || loading || marking"
        >
          <Check :size="15" />
          <span v-if="selectedIds.size > 0">Mark {{ selectedIds.size }} Read</span>
          <span v-else>Mark Read</span>
        </button>
      </div>
    </div>

    <!-- SwiftUI-style List -->
    <div class="list-content" v-if="!loading">
      <template v-if="emails.length > 0">
        <div
          v-for="email in emails"
          :key="email.id"
          class="list-row"
          :class="{ selected: selectedIds.has(email.id) }"
          @click="emit('toggle-select', email.id)"
        >
          <div class="row-checkbox">
            <input
              type="checkbox"
              :checked="selectedIds.has(email.id)"
              @click.stop
              @change="emit('toggle-select', email.id)"
            />
          </div>
          
          <div class="row-content">
            <div class="row-header">
              <span class="row-subject">{{ email.subject || "(No Subject)" }}</span>
              <span class="row-date">{{ formatDate(email.date_received) }}</span>
            </div>
            <div class="row-sender">{{ formatSender(email.sender) }}</div>
            <div class="row-footer" v-if="email.matchingFilters.length > 0">
              <span
                v-for="tag in email.matchingFilters"
                :key="tag"
                class="row-tag"
              >{{ tag }}</span>
            </div>
          </div>
          
          <div class="row-chevron">
            <ChevronRight :size="16" />
          </div>
        </div>
      </template>

      <!-- Empty State -->
      <div v-else class="empty-state">
        <div class="empty-icon">
          <Mail :size="48" :stroke-width="1" />
        </div>
        <p class="empty-title">No Messages</p>
        <p class="empty-subtitle">Messages matching your filters will appear here</p>
      </div>
    </div>

    <!-- Loading State -->
    <div v-else class="loading-state">
      <div class="loading-spinner"></div>
      <p class="loading-text">Loading messages...</p>
    </div>
  </div>
</template>

<style scoped>
.list-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* Progress Bar */
.progress-bar-container {
  padding: 10px 16px;
  background: var(--accent-light);
  border-bottom: 1px solid var(--accent-color);
}

.progress-bar-text {
  font-size: 12px;
  font-weight: 500;
  color: var(--accent-color);
  margin-bottom: 8px;
}

.progress-bar {
  height: 4px;
  background: rgba(0, 122, 255, 0.2);
  border-radius: 2px;
  overflow: hidden;
}

.progress-bar-fill {
  height: 100%;
  width: 30%;
  background: var(--accent-color);
  border-radius: 2px;
  animation: progress-slide 1s ease-in-out infinite;
}

@keyframes progress-slide {
  0% {
    transform: translateX(-100%);
    width: 30%;
  }
  50% {
    width: 50%;
  }
  100% {
    transform: translateX(400%);
    width: 30%;
  }
}

/* Toolbar - macOS style */
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--surface-tertiary);
  border-bottom: 1px solid var(--border-color);
  min-height: 44px;
}

.toolbar-leading,
.toolbar-trailing {
  display: flex;
  align-items: center;
  gap: 8px;
}

.checkbox-wrapper {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
}

.checkbox-wrapper input {
  width: 14px;
  height: 14px;
  accent-color: var(--accent-color);
  cursor: pointer;
}

.toolbar-count {
  font-size: 12px;
  color: var(--text-secondary);
  font-weight: 500;
}

.toolbar-button {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 8px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.12s ease;
}

.toolbar-button:hover:not(:disabled) {
  background: var(--control-bg);
  color: var(--text-color);
}

.toolbar-button:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.toolbar-button.primary {
  background: var(--accent-color);
  color: white;
  padding: 5px 12px;
}

.toolbar-button.primary:hover:not(:disabled) {
  filter: brightness(1.1);
  background: var(--accent-color);
}

.toolbar-divider {
  width: 1px;
  height: 20px;
  background: var(--border-color);
  margin: 0 4px;
}

/* List content */
.list-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

/* List row - SwiftUI style */
.list-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--separator-color);
  cursor: pointer;
  transition: background 0.1s ease;
}

.list-row:hover {
  background: var(--surface-hover);
}

.list-row.selected {
  background: var(--surface-selected);
}

.list-row:last-child {
  border-bottom: none;
}

.row-checkbox {
  flex-shrink: 0;
}

.row-checkbox input {
  width: 14px;
  height: 14px;
  accent-color: var(--accent-color);
  cursor: pointer;
}

.row-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.row-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.row-subject {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row-date {
  font-size: 12px;
  color: var(--text-tertiary);
  white-space: nowrap;
  flex-shrink: 0;
}

.row-sender {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1.4;
}

.row-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-top: 4px;
}

.row-tag {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 10px;
  background: var(--tag-bg);
  color: var(--tag-color);
  font-size: 10px;
  font-weight: 600;
}

.row-chevron {
  flex-shrink: 0;
  color: var(--text-tertiary);
  opacity: 0.5;
}

/* Empty state */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 48px 24px;
  text-align: center;
}

.empty-icon {
  color: var(--text-tertiary);
  margin-bottom: 16px;
  opacity: 0.5;
}

.empty-title {
  font-size: 17px;
  font-weight: 600;
  color: var(--text-color);
  margin: 0 0 4px;
}

.empty-subtitle {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}

/* Loading state */
.loading-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 48px;
}

.loading-spinner {
  width: 24px;
  height: 24px;
  border: 2.5px solid var(--border-color);
  border-top-color: var(--accent-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loading-text {
  font-size: 13px;
  color: var(--text-secondary);
  margin: 0;
}
</style>
