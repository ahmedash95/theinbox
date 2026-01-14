<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, Mail } from "lucide-vue-next";
import type { EmailWithMatches, EmailBody } from "../types";
import Button from "./ui/button.vue";
import Badge from "./ui/badge.vue";
import ScrollArea from "./ui/scroll-area.vue";

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
  <div class="flex h-full flex-col">
    <div class="border-b bg-muted/40 px-4 py-4">
      <Button variant="ghost" size="sm" @click="emit('back')">
        <ArrowLeft :size="16" />
        Back
      </Button>

      <div class="mt-4 space-y-2">
        <h2 class="text-lg font-semibold">{{ email.subject || "(No Subject)" }}</h2>
        <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <span class="font-medium text-foreground">{{ formatSender(email.sender) }}</span>
          <span>•</span>
          <span>{{ formatDate(email.date_received) }}</span>
        </div>
        <div v-if="email.matchingFilters.length > 0" class="flex flex-wrap gap-1">
          <Badge v-for="tag in email.matchingFilters" :key="tag" variant="secondary">
            {{ tag }}
          </Badge>
        </div>
      </div>
    </div>

    <ScrollArea class="flex-1">
      <div class="p-6">
        <div v-if="loading" class="flex flex-col items-center gap-2 text-sm text-muted-foreground">
          <div class="h-5 w-5 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground"></div>
          Loading email...
        </div>
        <div v-else-if="error" class="rounded-md border border-destructive/20 bg-destructive/5 p-4 text-sm text-destructive">
          <div class="flex items-center gap-2">
            <Mail :size="20" />
            <span>Failed to load email</span>
          </div>
          <div class="mt-2 text-xs text-destructive/80">{{ error }}</div>
        </div>
        <div v-else>
          <div
            v-if="emailBody?.html"
            class="text-sm leading-relaxed text-foreground [&_a]:text-primary [&_a]:underline [&_p]:mb-3"
            v-html="emailBody.html"
          ></div>
          <pre v-else-if="emailBody?.text" class="whitespace-pre-wrap text-sm text-foreground">
{{ emailBody.text }}
          </pre>
          <div v-else class="flex flex-col items-center gap-2 text-sm text-muted-foreground">
            <Mail :size="32" />
            No content available
          </div>
        </div>
      </div>
    </ScrollArea>
  </div>
</template>
