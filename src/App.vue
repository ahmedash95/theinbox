<script setup lang="ts">
import { ref, onMounted, watch, nextTick, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import FilterList from "./components/FilterList.vue";
import EmailList from "./components/EmailList.vue";
import EmailDetail from "./components/EmailDetail.vue";
import SettingsModal from "./components/SettingsModal.vue";
import Button from "./components/ui/button.vue";
import Badge from "./components/ui/badge.vue";
import type { Email, FilterPattern, EmailWithMatches, GmailEmail } from "./types";

// Settings state
const showSettings = ref(false);
const gmailEmail = ref<string | null>(null);

// Load settings from localStorage
function loadSettings() {
  const saved = localStorage.getItem("inboxcleanup_settings");
  if (saved) {
    try {
      const settings = JSON.parse(saved);
      gmailEmail.value = settings.gmail_email || null;
    } catch {
      gmailEmail.value = null;
    }
  }
}

// Save settings to localStorage
function saveSettings(newGmailEmail: string) {
  gmailEmail.value = newGmailEmail || null;
  localStorage.setItem(
    "inboxcleanup_settings",
    JSON.stringify({
      gmail_email: newGmailEmail || null,
    })
  );
  showSettings.value = false;
  // Refresh emails with updated Gmail settings
  refreshEmails();
}

// Window drag functionality
function startDrag(e: MouseEvent) {
  // Only start drag on left mouse button
  if (e.button !== 0) return;

  // Don't drag if clicking on interactive elements
  const target = e.target as HTMLElement;
  if (target.closest("button, input, a, select, [data-no-drag]")) return;

  // Prevent default to avoid text selection
  e.preventDefault();

  // Start dragging immediately
  getCurrentWindow().startDragging();
}

const filters = ref<FilterPattern[]>([]);
const allEmails = ref<Email[]>([]);
const selectedIds = ref<Set<string>>(new Set());
const loading = ref(true);
const marking = ref(false);
const markingCount = ref(0);
const error = ref<string | null>(null);
const viewingEmail = ref<EmailWithMatches | null>(null);


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

// Get enabled filters to apply
const filtersToApply = computed(() => {
  return filters.value.filter((f) => f.enabled);
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

// Computed: emails to display based on active filters
const displayedEmails = computed((): EmailWithMatches[] => {
  if (filtersToApply.value.length === 0) {
    return emailsWithMatches.value;
  }
  return emailsWithMatches.value.filter((e) => e.matchingFilters.length > 0);
});

// Load filters and emails on mount - but don't block UI
onMounted(async () => {
  await nextTick();
  loadSettings();
  await loadFilters();

  // Show settings if Gmail is not configured
  if (!gmailEmail.value) {
    showSettings.value = true;
    loading.value = false;
  } else {
    await refreshEmails();
  }
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
  if (!gmailEmail.value) {
    showSettings.value = true;
    return;
  }

  console.log("[UI] Refreshing emails via Gmail");
  loading.value = true;
  error.value = null;
  selectedIds.value = new Set();

  try {
    // Fetch from Gmail via IMAP
    const gmailEmails = await invoke<GmailEmail[]>("gmail_fetch_unread", {
      email: gmailEmail.value,
    });
    // Convert GmailEmail to Email format
    allEmails.value = gmailEmails.map((e) => ({
      id: e.uid.toString(),
      message_id: e.message_id,
      subject: e.subject,
      sender: e.sender,
      date_received: e.date,
      mailbox: "INBOX",
      account: gmailEmail.value!,
    }));
    console.log("[UI] Got", allEmails.value.length, "unread Gmail emails");
  } catch (e) {
    console.error("Failed to fetch emails:", e);
    error.value = String(e);
    allEmails.value = [];
  } finally {
    loading.value = false;
  }
}

async function forceRefresh() {
  if (!gmailEmail.value) {
    showSettings.value = true;
    return;
  }

  console.log("[UI] Force refreshing emails via Gmail");
  loading.value = true;
  error.value = null;
  selectedIds.value = new Set();

  try {
    // Gmail always fetches fresh (no cache)
    const gmailEmails = await invoke<GmailEmail[]>("gmail_fetch_unread", {
      email: gmailEmail.value,
    });
    allEmails.value = gmailEmails.map((e) => ({
      id: e.uid.toString(),
      message_id: e.message_id,
      subject: e.subject,
      sender: e.sender,
      date_received: e.date,
      mailbox: "INBOX",
      account: gmailEmail.value!,
    }));
    console.log("[UI] Force refreshed", allEmails.value.length, "Gmail emails");
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
  if (!gmailEmail.value) {
    showSettings.value = true;
    return;
  }

  marking.value = true;
  markingCount.value = selectedIds.value.size;
  error.value = null;

  try {
    const ids = Array.from(selectedIds.value);
    // Gmail uses UIDs (numbers)
    const uids = ids.map((id) => parseInt(id, 10));
    const count = await invoke<number>("gmail_mark_as_read", {
      email: gmailEmail.value,
      uids,
    });
    console.log(`Marked ${count} Gmail emails as read`);

    // Remove marked emails from local list immediately for snappy UI
    const markedSet = new Set(ids);
    allEmails.value = allEmails.value.filter((e) => !markedSet.has(e.id));
    selectedIds.value = new Set();
  } catch (e) {
    console.error("Failed to mark as read:", e);
    error.value = String(e);
  } finally {
    marking.value = false;
    markingCount.value = 0;
  }
}

function viewEmail(email: EmailWithMatches) {
  viewingEmail.value = email;
}

function backToList() {
  viewingEmail.value = null;
}

// Handle browser back button
if (typeof window !== "undefined") {
  window.addEventListener("popstate", () => {
    if (viewingEmail.value) {
      viewingEmail.value = null;
    }
  });
}
</script>

<template>
  <div class="flex h-screen flex-col bg-background text-foreground">
    <!-- Settings Modal -->
    <SettingsModal
      :show="showSettings"
      :current-gmail-email="gmailEmail"
      @close="showSettings = false"
      @save="saveSettings"
    />

    <!-- Titlebar -->
    <header
      class="relative flex h-12 items-center justify-end border-b bg-background/80 pl-16 pr-4 backdrop-blur"
      @mousedown="startDrag"
      data-tauri-drag-region
    >
      <div class="pointer-events-none absolute left-1/2 flex -translate-x-1/2 items-center gap-3">
        <span class="text-sm font-semibold">InboxCleanup</span>
        <Badge v-if="gmailEmail" variant="secondary">Gmail</Badge>
      </div>
      <Button
        variant="ghost"
        size="icon"
        @click="showSettings = true"
        data-no-drag
        aria-label="Settings"
      >
        ⚙️
      </Button>
    </header>

    <!-- Layout -->
    <main class="flex flex-1 overflow-hidden">
      <aside class="w-72 shrink-0 border-r bg-muted/30">
        <FilterList :filters="filters" :emails="allEmails" @update="saveFilters" />
      </aside>

      <section class="flex flex-1 flex-col overflow-hidden">
        <template v-if="!viewingEmail">

          <div
            v-if="error"
            class="mx-4 mt-3 flex items-center justify-between rounded-md border border-destructive/20 bg-destructive/5 px-3 py-2 text-sm text-destructive"
          >
            <span>{{ error }}</span>
            <Button variant="ghost" size="icon" @click="error = null" aria-label="Dismiss">
              ×
            </Button>
          </div>

          <div class="flex-1 overflow-hidden">
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
              @view-email="viewEmail"
            />
          </div>
        </template>

        <EmailDetail
          v-else
          :email="viewingEmail"
          :gmail-email="gmailEmail"
          @back="backToList"
        />
      </section>
    </main>
  </div>
</template>
