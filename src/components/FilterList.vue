<script setup lang="ts">
import { ref, computed } from "vue";
import { Plus, CheckSquare, Square } from "lucide-vue-next";
import type { FilterPattern, Email } from "../types";
import FilterModal from "./FilterModal.vue";
import Button from "./ui/button.vue";
import Checkbox from "./ui/checkbox.vue";
import ScrollArea from "./ui/scroll-area.vue";

const props = defineProps<{
  filters: FilterPattern[];
  emails: Email[];
  filterCounts: Record<string, number>;
}>();

const emit = defineEmits<{
  (e: "update", filters: FilterPattern[]): void;
}>();

const showModal = ref(false);
const editingFilter = ref<FilterPattern | null>(null);

const visibleFilters = computed(() => props.filters);

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

function handleDelete(filterId: number) {
  const updated = props.filters.filter((f) => f.id !== filterId);
  emit("update", updated);
  showModal.value = false;
}

function toggleFilter(id: number, enabled: boolean) {
  const updated = props.filters.map((f) =>
    f.id === id ? { ...f, enabled } : f
  );
  emit("update", updated);
}

function setAllVisible(enabled: boolean) {
  if (!props.filters.length) return;
  const updated = props.filters.map((filter) => ({ ...filter, enabled }));
  emit("update", updated);
}

function getFilterCount(filterId: number): number {
  return props.filterCounts[String(filterId)] ?? 0;
}

function formatCountShort(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}m`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return `${count}`;
}
</script>

<template>
  <div class="flex h-full flex-col gap-2 px-2 py-3">
    <div class="flex items-center justify-between px-1">
      <p class="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">Filters</p>
      <div class="flex items-center gap-0.5">
        <Button
          size="icon"
          variant="ghost"
          :disabled="visibleFilters.length === 0"
          @click="setAllVisible(true)"
          aria-label="Select all filters"
          class="h-8 w-8"
        >
          <CheckSquare :size="16" />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          :disabled="visibleFilters.length === 0"
          @click="setAllVisible(false)"
          aria-label="Deselect all filters"
          class="h-8 w-8"
        >
          <Square :size="16" />
        </Button>
      </div>
    </div>
    <div class="flex-1 overflow-hidden">
      <ScrollArea class="h-full">
        <div v-if="visibleFilters.length" class="grid gap-1 pb-3">
          <div
            v-for="filter in visibleFilters"
            :key="filter.id"
            class="flex h-9 items-center gap-2 rounded-md px-2 text-xs transition hover:bg-accent"
            :class="{ 'bg-muted/60 text-foreground': filter.enabled, 'text-muted-foreground': !filter.enabled }"
            @click="toggleFilter(filter.id, !filter.enabled)"
            @contextmenu.prevent="openEditModal(filter)"
          >
            <Checkbox
              class="h-4 w-4"
              :checked="filter.enabled"
              @update:checked="(value) => toggleFilter(filter.id, Boolean(value))"
              @click.stop
            />
            <span class="min-w-0 flex-1 truncate font-medium">{{ filter.name }}</span>
            <span class="text-[11px] text-muted-foreground">
              {{ formatCountShort(getFilterCount(filter.id)) }}
            </span>
          </div>
        </div>

        <div v-else class="flex h-full flex-col items-center justify-center gap-3 py-12 text-center">
          <div class="text-sm font-medium">No Filters</div>
          <div class="text-xs text-muted-foreground">
            Create filters to match and organize emails.
          </div>
          <Button size="sm" @click="openAddModal">
            <Plus :size="14" />
            New Filter
          </Button>
        </div>
      </ScrollArea>
    </div>
    <div class="px-1">
      <Button size="sm" variant="outline" class="w-full justify-start" @click="openAddModal">
        <Plus :size="14" />
        New Filter
      </Button>
    </div>

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
