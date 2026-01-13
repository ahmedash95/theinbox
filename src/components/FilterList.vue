<script setup lang="ts">
import { ref } from "vue";
import { Plus } from "lucide-vue-next";
import type { FilterPattern, Email } from "../types";
import FilterModal from "./FilterModal.vue";

const props = defineProps<{
  filters: FilterPattern[];
  emails: Email[];
}>();

const emit = defineEmits<{
  (e: "update", filters: FilterPattern[]): void;
}>();

const showModal = ref(false);
const editingFilter = ref<FilterPattern | null>(null);

function openAddModal() {
  editingFilter.value = null;
  showModal.value = true;
}

function openEditModal(filter: FilterPattern) {
  editingFilter.value = filter;
  showModal.value = true;
}

function handleSave(filter: FilterPattern) {
  if (editingFilter.value) {
    const updated = props.filters.map((f) =>
      f.id === filter.id ? filter : f
    );
    emit("update", updated);
  } else {
    emit("update", [...props.filters, filter]);
  }
}

function handleDelete(filterId: string) {
  const updated = props.filters.filter((f) => f.id !== filterId);
  emit("update", updated);
  showModal.value = false;
}

function toggleFilter(id: string, event: Event) {
  event.stopPropagation();
  const updated = props.filters.map((f) =>
    f.id === id ? { ...f, enabled: !f.enabled } : f
  );
  emit("update", updated);
}

// Calculate match count for each filter
function getMatchCount(filter: FilterPattern): number {
  if (!props.emails || props.emails.length === 0) return 0;
  
  return props.emails.filter((email) => {
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
  }).length;
}
</script>

<template>
  <div class="sidebar-content">
    <!-- Section Header -->
    <div class="section-header">
      <span class="section-title">Filters</span>
      <button class="add-button" @click="openAddModal" title="Add Filter">
        <Plus :size="14" :stroke-width="2.5" />
      </button>
    </div>

    <!-- Filter List -->
    <div class="filter-list">
      <div
        v-for="filter in filters"
        :key="filter.id"
        class="filter-row"
        :class="{ disabled: !filter.enabled }"
        @click="openEditModal(filter)"
      >
        <input
          type="checkbox"
          :checked="filter.enabled"
          @click="toggleFilter(filter.id, $event)"
          class="filter-checkbox"
        />
        <span class="filter-name">{{ filter.name }}</span>
        <span class="filter-count" v-if="filter.enabled">
          {{ getMatchCount(filter) }}
        </span>
      </div>

      <!-- Empty State -->
      <div v-if="filters.length === 0" class="empty-state">
        <p class="empty-title">No Filters</p>
        <p class="empty-subtitle">Create filters to match and organize emails</p>
        <button class="empty-button" @click="openAddModal">
          <Plus :size="14" :stroke-width="2.5" />
          New Filter
        </button>
      </div>
    </div>

    <!-- Modal -->
    <FilterModal
      :show="showModal"
      :edit-filter="editingFilter"
      :emails="emails"
      @close="showModal = false"
      @save="handleSave"
      @delete="handleDelete"
    />
  </div>
</template>

<style scoped>
.sidebar-content {
  display: flex;
  flex-direction: column;
  height: 100%;
}

/* Section Header */
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 12px 10px;
}

.section-title {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.add-button {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 6px;
  background: var(--accent-color);
  color: white;
  cursor: pointer;
  transition: all 0.15s ease;
}

.add-button:hover {
  transform: scale(1.08);
}

/* Filter List */
.filter-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 8px 8px;
}

/* Filter Row */
.filter-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 6px;
  margin-bottom: 2px;
  cursor: pointer;
  transition: all 0.12s ease;
}

.filter-row:hover {
  background: var(--surface-hover);
}

.filter-row.disabled {
  opacity: 0.5;
}

.filter-checkbox {
  width: 14px;
  height: 14px;
  accent-color: var(--accent-color);
  cursor: pointer;
  flex-shrink: 0;
}

.filter-name {
  flex: 1;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-color);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.filter-count {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-tertiary);
  background: var(--control-bg);
  padding: 2px 8px;
  border-radius: 10px;
  min-width: 24px;
  text-align: center;
}

/* Empty State */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 32px 16px;
  text-align: center;
}

.empty-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
  margin: 0 0 4px;
}

.empty-subtitle {
  font-size: 12px;
  color: var(--text-secondary);
  margin: 0 0 16px;
}

.empty-button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  background: var(--accent-color);
  color: white;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.empty-button:hover {
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(0, 122, 255, 0.3);
}
</style>
