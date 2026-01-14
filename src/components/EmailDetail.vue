<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, Mail } from "lucide-vue-next";
import type { EmailWithMatches, EmailBody } from "../types";

const props = defineProps<{
  email: EmailWithMatches;
  gmailEmail: string | null;
}>();

const emit = defineEmits<{
  (e: "back"): void;
}>();

const emailBody = ref<EmailBody | null>(null);
const loading = ref(true);
const error = ref<string | null>(null);

onMounted(async () => {
  try {
    if (!props.gmailEmail) {
      throw new Error("Gmail account not configured.");
    }
    const uid = parseInt(props.email.id, 10);
    emailBody.value = await invoke<EmailBody>("gmail_fetch_body", {
      email: props.gmailEmail,
      uid,
    });
  } catch (e) {
    console.error("Failed to fetch email body:", e);
    error.value = String(e);
  } finally {
    loading.value = false;
  }
});

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
    return date.toLocaleString([], {
      weekday: "short",
      year: "numeric",
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return dateStr;
  }
}
</script>

<template>
  <div class="email-detail-page">
    <!-- Header with back button -->
    <div class="detail-header">
      <button class="back-button" @click="emit('back')" title="Back to list">
        <ArrowLeft :size="20" />
        <span>Back</span>
      </button>
      <div class="header-content">
        <h2 class="subject">{{ email.subject || "(No Subject)" }}</h2>
        <div class="metadata">
          <div class="sender">{{ formatSender(email.sender) }}</div>
          <div class="date">{{ formatDate(email.date_received) }}</div>
        </div>
        <div v-if="email.matchingFilters.length > 0" class="filter-tags">
          <span
            v-for="tag in email.matchingFilters"
            :key="tag"
            class="filter-tag"
          >{{ tag }}</span>
        </div>
      </div>
    </div>

    <!-- Body -->
    <div class="detail-body">
      <div v-if="loading" class="loading-state">
        <div class="loading-spinner"></div>
        <p>Loading email...</p>
      </div>
      <div v-else-if="error" class="error-state">
        <Mail :size="48" :stroke-width="1" />
        <p class="error-title">Failed to Load Email</p>
        <p class="error-message">{{ error }}</p>
      </div>
      <div v-else class="body-content">
        <!-- Render HTML if available -->
        <div v-if="emailBody?.html" class="html-content" v-html="emailBody.html"></div>
        <!-- Fallback to plain text -->
        <pre v-else-if="emailBody?.text" class="plain-content">{{ emailBody.text }}</pre>
        <!-- No content -->
        <div v-else class="empty-content">
          <Mail :size="48" :stroke-width="1" />
          <p>No content available</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.email-detail-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.detail-header {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px 20px;
  border-bottom: 1px solid var(--separator-color);
  background: var(--surface-tertiary);
  flex-shrink: 0;
}

.back-button {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  background: var(--control-bg);
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
  align-self: flex-start;
}

.back-button:hover {
  background: var(--control-hover);
  color: var(--text-color);
}

.header-content {
  flex: 1;
  min-width: 0;
}

.subject {
  font-size: 17px;
  font-weight: 600;
  color: var(--text-color);
  margin: 0 0 8px;
  word-wrap: break-word;
}

.metadata {
  display: flex;
  gap: 12px;
  align-items: baseline;
  margin-bottom: 8px;
}

.sender {
  font-size: 13px;
  font-weight: 500;
  color: var(--text-color);
}

.date {
  font-size: 12px;
  color: var(--text-tertiary);
}

.filter-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}

.filter-tag {
  display: inline-flex;
  align-items: center;
  padding: 3px 10px;
  border-radius: 10px;
  background: var(--tag-bg);
  color: var(--tag-color);
  font-size: 11px;
  font-weight: 600;
}

.detail-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
  background: var(--surface-primary);
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 48px 24px;
  text-align: center;
  color: var(--text-secondary);
}

.loading-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--border-color);
  border-top-color: var(--accent-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.error-state {
  color: var(--text-tertiary);
}

.error-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-color);
  margin: 0;
}

.error-message {
  font-size: 13px;
  color: var(--danger-color);
  margin: 4px 0 0;
}

.body-content {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-color);
}

/* HTML email rendering */
.html-content {
  background: white;
  padding: 20px;
  border-radius: 8px;
  border: 1px solid var(--separator-color);
  overflow-x: auto;
}

.html-content :deep(*) {
  max-width: 100%;
}

.html-content :deep(img) {
  max-width: 100%;
  height: auto;
}

.html-content :deep(table) {
  border-collapse: collapse;
  max-width: 100%;
}

.html-content :deep(a) {
  color: var(--accent-color);
  text-decoration: none;
}

.html-content :deep(a:hover) {
  text-decoration: underline;
}

@media (prefers-color-scheme: dark) {
  .html-content {
    background: var(--surface-secondary);
  }
}

/* Plain text fallback */
.plain-content {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "SF Mono",
    "Helvetica Neue", Helvetica, Arial, sans-serif;
  white-space: pre-wrap;
  word-wrap: break-word;
  padding: 20px;
  background: var(--surface-secondary);
  border-radius: 8px;
  border: 1px solid var(--separator-color);
}
</style>
