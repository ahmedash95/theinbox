<script setup lang="ts">
import { computed } from "vue";
import { Check, CheckSquare, MinusSquare, Square, Mail } from "lucide-vue-next";
import type { EmailWithMatches } from "../types";
import Button from "./ui/button.vue";
import Badge from "./ui/badge.vue";
import Checkbox from "./ui/checkbox.vue";
import ScrollArea from "./ui/scroll-area.vue";

const props = defineProps<{
  emails: EmailWithMatches[];
  loading: boolean;
  marking: boolean;
  markingCount: number;
  selectedIds: Set<string>;
  page: number;
  pageCount: number;
  canPrev: boolean;
  canNext: boolean;
}>();

const emit = defineEmits<{
  (e: "toggle-select", id: string): void;
  (e: "select-all"): void;
  (e: "deselect-all"): void;
  (e: "mark-read"): void;
  (e: "view-email", email: EmailWithMatches): void;
  (e: "prev-page"): void;
  (e: "next-page"): void;
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
  let cleaned = sender.replace(/^["']|["']$/g, "").trim();

  // Extract name from "Name <email@example.com>" format
  const match = cleaned.match(/^(.+?)\s*<.+>$/);
  if (match) {
    cleaned = match[1].trim();
  }

  // Remove any remaining quotes
  cleaned = cleaned.replace(/^["']|["']$/g, "").trim();

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
  <div class="flex h-full flex-col">
    <div v-if="marking" class="border-b bg-primary/5 px-4 py-3 text-xs text-primary">
      Marking {{ markingCount }} {{ markingCount === 1 ? "email" : "emails" }} as read...
      <div class="mt-2 h-1 w-full overflow-hidden rounded-full bg-primary/15">
        <div class="h-full w-1/3 animate-pulse rounded-full bg-primary"></div>
      </div>
    </div>

    <div class="flex items-center border-b px-4 py-2">
      <div class="flex items-center gap-2">
        <Button
          variant="ghost"
          size="icon"
          @click="allSelected ? emit('deselect-all') : emit('select-all')"
          :disabled="emails.length === 0 || loading || marking"
          aria-label="Toggle select all"
          title="Select all"
        >
          <CheckSquare v-if="allSelected" :size="16" />
          <MinusSquare v-else-if="someSelected" :size="16" />
          <Square v-else :size="16" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          @click="emit('mark-read')"
          :disabled="selectedIds.size === 0 || loading || marking"
          aria-label="Mark selected as read"
          title="Mark selected as read"
        >
          <Check :size="16" />
        </Button>
      </div>
    </div>

    <div v-if="loading" class="flex flex-1 items-center justify-center">
      <div class="flex flex-col items-center gap-2 text-sm text-muted-foreground">
        <div class="h-5 w-5 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground"></div>
        Loading messages...
      </div>
    </div>

    <ScrollArea v-else class="flex-1">
      <div v-if="emails.length" class="divide-y">
        <div
          v-for="email in emails"
          :key="email.id"
          class="flex items-start gap-3 px-4 py-3 transition hover:bg-muted/40"
          :class="{ 'bg-muted/40': selectedIds.has(email.id) }"
        >
          <Checkbox
            class="mt-1"
            :checked="selectedIds.has(email.id)"
            @update:checked="() => emit('toggle-select', email.id)"
          />

          <button class="flex min-w-0 flex-1 flex-col gap-1 text-left" @click="emit('view-email', email)">
            <div class="flex items-center justify-between gap-3">
              <span class="truncate text-sm font-medium">{{ email.subject || "(No Subject)" }}</span>
              <div class="flex items-center gap-2">
                <div v-if="email.matchingFilters.length > 0" class="flex max-w-[140px] flex-wrap justify-end gap-1">
                  <Badge v-for="tag in email.matchingFilters" :key="tag" variant="secondary">
                    {{ tag }}
                  </Badge>
                </div>
                <span class="shrink-0 text-xs text-muted-foreground">{{ formatDate(email.date_received) }}</span>
              </div>
            </div>
            <span class="truncate text-xs text-muted-foreground">{{ formatSender(email.sender) }}</span>
          </button>

        </div>
      </div>

      <div v-else class="flex h-full flex-col items-center justify-center gap-2 py-16 text-center">
        <Mail :size="36" class="text-muted-foreground" />
        <p class="text-sm font-medium">No Messages</p>
        <p class="text-xs text-muted-foreground">
          Messages matching your filters will appear here.
        </p>
      </div>
    </ScrollArea>

    <div class="flex items-center justify-between border-t px-4 py-2 text-xs text-muted-foreground">
      <span>Page {{ page }} of {{ pageCount }}</span>
      <div class="flex items-center gap-2">
        <Button variant="ghost" size="sm" :disabled="!canPrev" @click="emit('prev-page')">
          Prev
        </Button>
        <Button variant="ghost" size="sm" :disabled="!canNext" @click="emit('next-page')">
          Next
        </Button>
      </div>
    </div>
  </div>
</template>
