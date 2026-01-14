<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import Button from "./ui/button.vue";
import Input from "./ui/input.vue";
import Badge from "./ui/badge.vue";

const props = defineProps<{
  show: boolean;
  currentGmailEmail: string | null;
}>();

const emit = defineEmits<{
  close: [];
  save: [gmailEmail: string];
}>();

const gmailEmail = ref(props.currentGmailEmail || "");
const gmailAppPassword = ref("");
const testing = ref(false);
const saving = ref(false);
const testResult = ref<{ success: boolean; message: string } | null>(null);
const isConfigured = ref(false);

// Check if Gmail is already configured when email changes
async function checkGmailConfigured() {
  if (gmailEmail.value && gmailEmail.value.includes("@")) {
    try {
      isConfigured.value = await invoke<boolean>("gmail_is_configured", {
        email: gmailEmail.value,
      });
    } catch {
      isConfigured.value = false;
    }
  } else {
    isConfigured.value = false;
  }
}

watch(
  () => props.show,
  (visible) => {
    if (visible) {
      checkGmailConfigured();
    }
  }
);

watch(
  () => props.currentGmailEmail,
  (value) => {
    if (value !== gmailEmail.value) {
      gmailEmail.value = value || "";
    }
  }
);

const canSave = computed(() => {
  return (
    gmailEmail.value.includes("@") &&
    (isConfigured.value || gmailAppPassword.value.length >= 16)
  );
});

async function testConnection() {
  if (!gmailEmail.value || !gmailAppPassword.value) return;

  testing.value = true;
  testResult.value = null;

  try {
    const result = await invoke<string>("gmail_test_connection", {
      email: gmailEmail.value,
      appPassword: gmailAppPassword.value,
    });
    testResult.value = { success: true, message: result };
  } catch (e) {
    testResult.value = { success: false, message: String(e) };
  } finally {
    testing.value = false;
  }
}

async function handleSave() {
  if (!canSave.value) return;

  saving.value = true;

  try {
    if (gmailAppPassword.value) {
      // Store credentials in keychain
      await invoke("gmail_store_credentials", {
        email: gmailEmail.value,
        appPassword: gmailAppPassword.value,
      });
    }

    emit("save", gmailEmail.value);
  } catch (e) {
    testResult.value = { success: false, message: String(e) };
  } finally {
    saving.value = false;
  }
}

async function removeGmailAccount() {
  if (!gmailEmail.value) return;

  try {
    await invoke("gmail_delete_credentials", { email: gmailEmail.value });
    isConfigured.value = false;
    gmailAppPassword.value = "";
    testResult.value = { success: true, message: "Account removed from Keychain" };
  } catch (e) {
    testResult.value = { success: false, message: String(e) };
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm"
      @click.self="emit('close')"
    >
      <div class="w-full max-w-md overflow-hidden rounded-lg border bg-card shadow-xl">
        <div class="flex items-center justify-between border-b px-5 py-4">
          <h2 class="text-sm font-semibold">Email Settings</h2>
          <Button variant="ghost" size="icon" @click="emit('close')" aria-label="Close">
            ×
          </Button>
        </div>

        <div class="space-y-5 px-5 py-4">
          <div class="space-y-2">
            <label class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Gmail Account
            </label>
            <Input
              v-model="gmailEmail"
              type="email"
              placeholder="you@gmail.com"
              @blur="checkGmailConfigured"
            />
          </div>

          <div v-if="isConfigured" class="flex items-center gap-2 rounded-md border bg-muted/40 px-3 py-2">
            <Badge variant="secondary">Configured</Badge>
            <span class="text-xs text-muted-foreground">Password stored in Keychain</span>
            <Button variant="ghost" size="sm" class="ml-auto" @click="removeGmailAccount">
              Remove
            </Button>
          </div>

          <div v-if="!isConfigured" class="space-y-2">
            <label class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              App Password
            </label>
            <Input
              v-model="gmailAppPassword"
              type="password"
              placeholder="16-character app password"
              maxlength="19"
            />
            <div class="text-xs text-muted-foreground">
              Requires 2-Step Verification.
              <a
                class="text-primary underline-offset-4 hover:underline"
                href="https://myaccount.google.com/apppasswords"
                target="_blank"
              >
                Generate one
              </a>
            </div>
          </div>

          <div v-if="!isConfigured && gmailEmail && gmailAppPassword" class="flex items-center gap-2">
            <Button variant="outline" size="sm" :disabled="testing" @click="testConnection">
              {{ testing ? "Testing..." : "Test Connection" }}
            </Button>
          </div>

          <div
            v-if="testResult"
            class="rounded-md border px-3 py-2 text-xs"
            :class="testResult.success ? 'border-emerald-200 bg-emerald-50 text-emerald-700' : 'border-destructive/30 bg-destructive/5 text-destructive'"
          >
            {{ testResult.message }}
          </div>
        </div>

        <div class="flex justify-end gap-2 border-t px-5 py-4">
          <Button variant="outline" @click="emit('close')">Cancel</Button>
          <Button :disabled="!canSave || saving" @click="handleSave">
            {{ saving ? "Saving..." : "Save" }}
          </Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
