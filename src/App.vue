<script setup lang="ts">
import { ref, onMounted, watch, nextTick, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import FilterList from "./components/FilterList.vue";
import EmailList from "./components/EmailList.vue";
import type { Email, FilterPattern, EmailWithMatches } from "./types";

// Window drag functionality
function startDrag(e: MouseEvent) {
  // Only start drag on left mouse button
  if (e.button !== 0) return;
  
  // Don't drag if clicking on interactive elements
  const target = e.target as HTMLElement;
  if (target.closest('button, input, a, select, [data-no-drag]')) return;
  
  // Prevent default to avoid text selection
  e.preventDefault();
  
  // Start dragging immediately
  getCurrentWindow().startDragging();
}

const filters = ref<FilterPattern[]>([]);
const allEmails = ref<Email[]>([]);
const selectedIds = ref<Set<string>>(new Set());
const loading = ref(true);
const initialLoad = ref(true);
const marking = ref(false);
const markingCount = ref(0);
const error = ref<string | null>(null);

// View mode: 'all' shows all unread, 'filtered' shows only matching filters
const viewMode = ref<"all" | "filtered">("all");

// Selected filter IDs for filtered view (empty = all enabled)
const activeFilterIds = ref<Set<string>>(new Set());

// Helper to check if email matches a filter
function emailMatchesFilter(email: Email, filter: FilterPattern): boolean {
  try {
    if (filter.is_regex) {
      const regex = new RegExp(filter.pattern, "i");
      switch (filter.field) {
        case "subject":
          return regex.test(email.subject);
        case "sender":
          return regex.test(email.sender);
        case "any":
          return regex.test(email.subject) || regex.test(email.sender);
        default:
          return false;
      }
    } else {
      const patternLower = filter.pattern.toLowerCase();
      switch (filter.field) {
        case "subject":
          return email.subject.toLowerCase().includes(patternLower);
        case "sender":
          return email.sender.toLowerCase().includes(patternLower);
        case "any":
          return (
            email.subject.toLowerCase().includes(patternLower) ||
            email.sender.toLowerCase().includes(patternLower)
          );
        default:
          return false;
      }
    }
  } catch {
    return false;
  }
}

// Get filters to apply based on selection
const filtersToApply = computed(() => {
  const enabled = filters.value.filter((f) => f.enabled);
  if (activeFilterIds.value.size === 0) {
    return enabled; // All enabled filters
  }
  return enabled.filter((f) => activeFilterIds.value.has(f.id));
});

// Computed: emails with matching filter names
const emailsWithMatches = computed((): EmailWithMatches[] => {
  return allEmails.value.map((email) => {
    const matchingFilters = filtersToApply.value
      .filter((filter) => emailMatchesFilter(email, filter))
      .map((f) => f.name);
    
    return {
      ...email,
      matchingFilters,
    };
  });
});

// Computed: emails to display based on view mode
const displayedEmails = computed((): EmailWithMatches[] => {
  if (viewMode.value === "all") {
    return emailsWithMatches.value;
  }
  return emailsWithMatches.value.filter((e) => e.matchingFilters.length > 0);
});

// Count of filtered emails
const filteredCount = computed(() => {
  return emailsWithMatches.value.filter((e) => e.matchingFilters.length > 0).length;
});

// Enabled filters for the filter bar
const enabledFilters = computed(() => filters.value.filter((f) => f.enabled));

// Toggle a filter in the active selection
function toggleActiveFilter(filterId: string) {
  const newSet = new Set(activeFilterIds.value);
  if (newSet.has(filterId)) {
    newSet.delete(filterId);
  } else {
    newSet.add(filterId);
  }
  activeFilterIds.value = newSet;
  selectedIds.value = new Set();
}

// Select all filters
function selectAllFilters() {
  activeFilterIds.value = new Set();
  selectedIds.value = new Set();
}

// Load filters and emails on mount - but don't block UI
onMounted(async () => {
  await nextTick();
  await loadFilters();
  await refreshEmails();
  initialLoad.value = false;
});

// When filters change, just re-compute (no need to re-fetch)
watch(
  filters,
  () => {
    selectedIds.value = new Set();
  },
  { deep: true }
);

async function loadFilters() {
  console.log("[UI] Loading filters...");
  try {
    filters.value = await invoke<FilterPattern[]>("get_filters");
    console.log("[UI] Loaded", filters.value.length, "filters");
  } catch (e) {
    console.error("Failed to load filters:", e);
    error.value = String(e);
  }
}

async function saveFilters(newFilters: FilterPattern[]) {
  try {
    await invoke("save_filter_patterns", { patterns: newFilters });
    filters.value = newFilters;
  } catch (e) {
    console.error("Failed to save filters:", e);
    error.value = String(e);
  }
}

async function refreshEmails() {
  console.log("[UI] Refreshing emails...");
  loading.value = true;
  error.value = null;
  selectedIds.value = new Set();

  try {
    allEmails.value = await invoke<Email[]>("get_unread_emails", { inboxOnly: true });
    console.log("[UI] Got", allEmails.value.length, "unread emails");
  } catch (e) {
    console.error("Failed to fetch emails:", e);
    error.value = String(e);
    allEmails.value = [];
  } finally {
    loading.value = false;
  }
}

async function forceRefresh() {
  console.log("[UI] Force refreshing emails...");
  loading.value = true;
  error.value = null;
  selectedIds.value = new Set();

  try {
    allEmails.value = await invoke<Email[]>("force_refresh", { inboxOnly: true });
    console.log("[UI] Force refreshed", allEmails.value.length, "unread emails");
  } catch (e) {
    console.error("Failed to refresh emails:", e);
    error.value = String(e);
    allEmails.value = [];
  } finally {
    loading.value = false;
  }
}

function toggleSelect(id: string) {
  const newSet = new Set(selectedIds.value);
  if (newSet.has(id)) {
    newSet.delete(id);
  } else {
    newSet.add(id);
  }
  selectedIds.value = newSet;
}

function selectAll() {
  selectedIds.value = new Set(displayedEmails.value.map((e) => e.id));
}

function deselectAll() {
  selectedIds.value = new Set();
}

async function markAsRead() {
  if (selectedIds.value.size === 0) return;

  marking.value = true;
  markingCount.value = selectedIds.value.size;
  error.value = null;

  try {
    const ids = Array.from(selectedIds.value);
    const count = await invoke<number>("mark_as_read", { emailIds: ids });
    console.log(`Marked ${count} emails as read`);
    
    // Remove marked emails from local list immediately for snappy UI
    const markedSet = new Set(ids);
    allEmails.value = allEmails.value.filter(e => !markedSet.has(e.id));
    selectedIds.value = new Set();
    
  } catch (e) {
    console.error("Failed to mark as read:", e);
    error.value = String(e);
  } finally {
    marking.value = false;
    markingCount.value = 0;
  }
}

function setViewMode(mode: "all" | "filtered") {
  viewMode.value = mode;
  selectedIds.value = new Set();
}
</script>

<template>
  <div class="app">
    <!-- Transparent draggable titlebar -->
    <div 
      class="titlebar" 
      @mousedown="startDrag"
      data-tauri-drag-region
    >
      <div class="titlebar-traffic-lights"></div>
      <div class="titlebar-title">InboxCleanup</div>
      <div class="titlebar-spacer"></div>
    </div>

    <!-- Navigation Split View -->
    <main class="split-view">
      <!-- Sidebar -->
      <aside class="split-sidebar">
        <FilterList :filters="filters" :emails="allEmails" @update="saveFilters" />
      </aside>
      
      <!-- Content -->
      <section class="split-content">
        <!-- Navigation Bar -->
        <nav class="navbar">
          <div class="navbar-title">
            <div class="segmented-control">
              <button
                :class="{ active: viewMode === 'all' }"
                @click="setViewMode('all')"
              >
                All
                <span class="segment-badge">{{ allEmails.length }}</span>
              </button>
              <button
                :class="{ active: viewMode === 'filtered' }"
                @click="setViewMode('filtered')"
              >
                Filtered
                <span class="segment-badge">{{ filteredCount }}</span>
              </button>
            </div>
          </div>
        </nav>

        <!-- Filter Pills (in filtered mode) -->
        <div v-if="viewMode === 'filtered' && enabledFilters.length > 0" class="filter-pills">
          <button
            class="pill"
            :class="{ active: activeFilterIds.size === 0 }"
            @click="selectAllFilters"
          >
            All Filters
          </button>
          <button
            v-for="filter in enabledFilters"
            :key="filter.id"
            class="pill"
            :class="{ active: activeFilterIds.has(filter.id) }"
            @click="toggleActiveFilter(filter.id)"
          >
            {{ filter.name }}
          </button>
        </div>

        <!-- Error Banner -->
        <div v-if="error" class="error-banner">
          <span>{{ error }}</span>
          <button @click="error = null">×</button>
        </div>

        <!-- Email List -->
        <EmailList
          :emails="displayedEmails"
          :loading="loading"
          :marking="marking"
          :marking-count="markingCount"
          :selected-ids="selectedIds"
          @toggle-select="toggleSelect"
          @select-all="selectAll"
          @deselect-all="deselectAll"
          @mark-read="markAsRead"
          @refresh="refreshEmails"
          @force-refresh="forceRefresh"
        />
      </section>
    </main>
  </div>
</template>

<style>
:root {
  /* System colors - Light mode */
  --text-color: #000000;
  --text-secondary: #3a3a3c;
  --text-tertiary: #6e6e73;
  --border-color: rgba(0, 0, 0, 0.15);
  --separator-color: rgba(0, 0, 0, 0.1);
  
  /* Surfaces - more opaque for readability on vibrancy */
  --surface-primary: rgba(255, 255, 255, 0.85);
  --surface-secondary: rgba(255, 255, 255, 0.75);
  --surface-tertiary: rgba(246, 246, 246, 0.8);
  --surface-hover: rgba(0, 0, 0, 0.06);
  --surface-selected: rgba(0, 122, 255, 0.15);
  
  /* Controls - more visible */
  --control-bg: rgba(120, 120, 128, 0.12);
  --control-hover: rgba(120, 120, 128, 0.2);
  --control-active: rgba(255, 255, 255, 0.95);
  
  /* Accent */
  --accent-color: #007aff;
  --accent-light: rgba(0, 122, 255, 0.12);
  --success-color: #34c759;
  --success-light: rgba(52, 199, 89, 0.15);
  --danger-color: #ff3b30;
  --danger-bg: rgba(255, 59, 48, 0.15);
  
  /* Tags */
  --tag-bg: rgba(0, 122, 255, 0.15);
  --tag-color: #0066cc;
}

@media (prefers-color-scheme: dark) {
  :root {
    /* Dark mode - brighter text for vibrancy backgrounds */
    --text-color: #ffffff;
    --text-secondary: #ebebf5;
    --text-tertiary: #ababb0;
    --border-color: rgba(255, 255, 255, 0.2);
    --separator-color: rgba(255, 255, 255, 0.12);
    
    /* Darker surfaces with more opacity */
    --surface-primary: rgba(28, 28, 30, 0.85);
    --surface-secondary: rgba(36, 36, 38, 0.8);
    --surface-tertiary: rgba(44, 44, 46, 0.75);
    --surface-hover: rgba(255, 255, 255, 0.08);
    --surface-selected: rgba(10, 132, 255, 0.3);
    
    /* Controls - more visible in dark mode */
    --control-bg: rgba(118, 118, 128, 0.24);
    --control-hover: rgba(118, 118, 128, 0.36);
    --control-active: rgba(72, 72, 74, 0.95);
    
    /* Brighter accent for dark mode */
    --accent-color: #0a84ff;
    --accent-light: rgba(10, 132, 255, 0.2);
    --success-color: #30d158;
    --success-light: rgba(48, 209, 88, 0.2);
    --danger-color: #ff453a;
    --danger-bg: rgba(255, 69, 58, 0.25);
    
    --tag-bg: rgba(10, 132, 255, 0.3);
    --tag-color: #64d2ff;
  }
}

* {
  box-sizing: border-box;
}

html, body {
  margin: 0;
  padding: 0;
  height: 100%;
  overflow: hidden;
  background: transparent !important;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Pro Display", 
    "Helvetica Neue", Helvetica, Arial, sans-serif;
  color: var(--text-color);
  font-size: 13px;
  line-height: 1.4;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

/* Titlebar */
.titlebar {
  height: 52px;
  display: flex;
  align-items: center;
  flex-shrink: 0;
  cursor: grab;
  user-select: none;
  -webkit-user-select: none;
  -webkit-app-region: drag;
  padding: 0 12px;
}

.titlebar:active {
  cursor: grabbing;
}

.titlebar-traffic-lights {
  width: 70px;
  flex-shrink: 0;
}

.titlebar-spacer {
  width: 70px;
  flex-shrink: 0;
}

.titlebar-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  text-align: center;
  flex: 1;
  text-shadow: 0 0 10px rgba(255, 255, 255, 0.3);
}

@media (prefers-color-scheme: dark) {
  .titlebar-title {
    text-shadow: 0 0 10px rgba(0, 0, 0, 0.3);
  }
}

/* Split View - macOS NavigationSplitView */
.split-view {
  display: flex;
  flex: 1;
  overflow: hidden;
  margin: 0 8px 8px;
  gap: 1px;
  background: var(--separator-color);
  border-radius: 10px;
  border: 1px solid var(--border-color);
}

.split-sidebar {
  width: 240px;
  flex-shrink: 0;
  background: var(--surface-primary);
  backdrop-filter: blur(30px) saturate(180%);
  -webkit-backdrop-filter: blur(30px) saturate(180%);
  overflow: hidden;
  border-radius: 10px 0 0 10px;
}

.split-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: var(--surface-secondary);
  backdrop-filter: blur(30px) saturate(180%);
  -webkit-backdrop-filter: blur(30px) saturate(180%);
  overflow: hidden;
  border-radius: 0 10px 10px 0;
}

/* Navigation Bar */
.navbar {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 10px 16px;
  background: var(--surface-tertiary);
  border-bottom: 1px solid var(--separator-color);
  min-height: 44px;
}

.navbar-title {
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Segmented Control - SwiftUI style */
.segmented-control {
  display: inline-flex;
  background: var(--control-bg);
  border-radius: 8px;
  padding: 2px;
}

.segmented-control button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 16px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.segmented-control button:hover:not(.active) {
  color: var(--text-secondary);
  background: rgba(0, 0, 0, 0.03);
}

.segmented-control button.active {
  background: var(--control-active);
  color: var(--text-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.08);
}

.segment-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 20px;
  height: 18px;
  padding: 0 6px;
  border-radius: 9px;
  font-size: 11px;
  font-weight: 600;
  background: var(--control-bg);
  color: var(--text-tertiary);
}

.segmented-control button.active .segment-badge {
  background: rgba(0, 0, 0, 0.08);
  color: var(--text-secondary);
}

@media (prefers-color-scheme: dark) {
  .segmented-control button:hover:not(.active) {
    background: rgba(255, 255, 255, 0.05);
  }
  
  .segmented-control button.active {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
  }
  
  .segmented-control button.active .segment-badge {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-secondary);
  }
}

/* Filter Pills */
.filter-pills {
  display: flex;
  gap: 6px;
  padding: 8px 16px;
  background: var(--surface-tertiary);
  border-bottom: 1px solid var(--separator-color);
  overflow-x: auto;
  flex-shrink: 0;
}

.filter-pills::-webkit-scrollbar {
  display: none;
}

.pill {
  display: inline-flex;
  align-items: center;
  padding: 6px 14px;
  border: 1px solid var(--border-color);
  border-radius: 16px;
  background: var(--surface-primary);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.15s ease;
}

.pill:hover:not(.active) {
  background: var(--control-hover);
  color: var(--text-color);
  border-color: var(--text-tertiary);
}

.pill.active {
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}

/* Error Banner */
.error-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  margin: 8px 16px;
  border-radius: 8px;
  background: var(--danger-bg);
  color: var(--danger-color);
  font-size: 12px;
  font-weight: 500;
}

.error-banner button {
  background: none;
  border: none;
  color: var(--danger-color);
  font-size: 16px;
  cursor: pointer;
  opacity: 0.7;
  padding: 0 4px;
}

.error-banner button:hover {
  opacity: 1;
}

/* Scrollbar */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.15);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.25);
}

@media (prefers-color-scheme: dark) {
  ::-webkit-scrollbar-thumb {
    background: rgba(255, 255, 255, 0.15);
  }
  
  ::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.25);
  }
}
</style>
