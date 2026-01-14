<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Database, Mail } from "lucide-vue-next";
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
const activeTab = ref<"account" | "storage">("account");
let removeKeyListener: (() => void) | null = null;

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
      activeTab.value = "account";
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

onMounted(() => {
  const handler = (event: KeyboardEvent) => {
    if (event.key === "Escape" && props.show) {
      emit("close");
    }
  };
  window.addEventListener("keydown", handler);
  removeKeyListener = () => window.removeEventListener("keydown", handler);
});

onUnmounted(() => {
  if (removeKeyListener) {
    removeKeyListener();
    removeKeyListener = null;
  }
});

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

async function openDatabaseFolder() {
  try {
    const filePath = await invoke<string>("get_db_file_path");
    await revealItemInDir(filePath);
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
      <div class="flex h-[500px] w-[500px] flex-col overflow-hidden rounded-lg border bg-card shadow-xl">
        <div class="px-5 py-4">
          <div class="inline-flex w-full items-center justify-center gap-2 text-xs">
            <button
              type="button"
              class="flex h-14 w-28 flex-col items-center justify-center gap-1 rounded-md border font-medium transition"
              :class="activeTab === 'account' ? 'border-transparent bg-background shadow-sm' : 'border-transparent text-muted-foreground'"
              @click="activeTab = 'account'"
            >
              <Mail :size="18" />
              Account
            </button>
            <button
              type="button"
              class="flex h-14 w-28 flex-col items-center justify-center gap-1 rounded-md border font-medium transition"
              :class="activeTab === 'storage' ? 'border-transparent bg-background shadow-sm' : 'border-transparent text-muted-foreground'"
              @click="activeTab = 'storage'"
            >
              <Database :size="18" />
              Storage
            </button>
          </div>
        </div>

        <div class="border-t"></div>

        <div class="flex-1 space-y-6 overflow-auto px-5 py-4">
          <template v-if="activeTab === 'account'">
            <div class="space-y-3">
              <div class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Gmail
              </div>
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
            </div>
          </template>

          <template v-else>
            <div class="space-y-3">
              <div class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Database
              </div>
              <div class="flex items-center justify-between rounded-md border bg-muted/40 px-3 py-2">
                <div>
                  <div class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    Local Database
                  </div>
                  <div class="text-xs text-muted-foreground">Open the SQLite folder in Finder.</div>
                </div>
                <Button variant="outline" size="sm" @click="openDatabaseFolder">
                  Open Folder
                </Button>
              </div>
            </div>
          </template>

          <div
            v-if="testResult"
            class="rounded-md border px-3 py-2 text-xs"
            :class="testResult.success ? 'border-emerald-200 bg-emerald-50 text-emerald-700' : 'border-destructive/30 bg-destructive/5 text-destructive'"
          >
            {{ testResult.message }}
          </div>
        </div>

        <div class="flex justify-end border-t px-5 py-4">
          <Button variant="outline" @click="emit('close')">Close</Button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
