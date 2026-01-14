<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
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
    <div v-if="show" class="modal-overlay" @click.self="emit('close')">
      <div class="modal">
        <div class="modal-header">
          <h2>Email Settings</h2>
          <button class="close-btn" @click="emit('close')">×</button>
        </div>

        <div class="modal-body">
          <!-- Gmail Configuration -->
          <div class="section">
            <label class="section-label">Gmail Account</label>

            <div class="form-group">
              <label>Email Address</label>
              <input
                v-model="gmailEmail"
                type="email"
                placeholder="you@gmail.com"
                @blur="checkGmailConfigured"
              />
            </div>

            <div v-if="isConfigured" class="configured-badge">
              ✓ Account configured (password in Keychain)
              <button class="link-btn danger" @click="removeGmailAccount">
                Remove
              </button>
            </div>

            <div v-if="!isConfigured" class="form-group">
              <label>
                App Password
                <a
                  href="https://myaccount.google.com/apppasswords"
                  target="_blank"
                  class="help-link"
                >
                  Generate one →
                </a>
              </label>
              <input
                v-model="gmailAppPassword"
                type="password"
                placeholder="16-character app password"
                maxlength="19"
              />
              <div class="help-text">
                Requires 2-Step Verification. Not your regular password.
              </div>
            </div>

            <!-- Test Connection -->
            <div v-if="!isConfigured && gmailEmail && gmailAppPassword" class="form-group">
              <button
                class="btn secondary"
                :disabled="testing"
                @click="testConnection"
              >
                {{ testing ? "Testing..." : "Test Connection" }}
              </button>
            </div>

            <!-- Result -->
            <div v-if="testResult" class="test-result" :class="{ success: testResult.success, error: !testResult.success }">
              {{ testResult.message }}
            </div>
          </div>
        </div>

        <div class="modal-footer">
          <button class="btn secondary" @click="emit('close')">Cancel</button>
          <button
            class="btn primary"
            :disabled="!canSave || saving"
            @click="handleSave"
          >
            {{ saving ? "Saving..." : "Save" }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  width: 480px;
  max-width: 90vw;
  max-height: 85vh;
  background: var(--surface-primary);
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--separator-color);
}

.modal-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.close-btn {
  background: none;
  border: none;
  font-size: 20px;
  color: var(--text-tertiary);
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
}

.close-btn:hover {
  background: var(--surface-hover);
  color: var(--text-color);
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.section {
  margin-bottom: 24px;
}

.section-label {
  display: block;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-tertiary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 12px;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.form-group input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--surface-tertiary);
  color: var(--text-color);
  font-size: 14px;
}

.form-group input:focus {
  outline: none;
  border-color: var(--accent-color);
  box-shadow: 0 0 0 3px var(--accent-light);
}

.help-link {
  font-size: 12px;
  color: var(--accent-color);
  text-decoration: none;
}

.help-link:hover {
  text-decoration: underline;
}

.help-text {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 6px;
}

.configured-badge {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: var(--success-light);
  color: var(--success-color);
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  margin-bottom: 16px;
}

.link-btn {
  background: none;
  border: none;
  font-size: 12px;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  margin-left: auto;
}

.link-btn.danger {
  color: var(--danger-color);
}

.link-btn:hover {
  background: var(--surface-hover);
}

.test-result {
  padding: 12px 16px;
  border-radius: 8px;
  font-size: 13px;
  margin-top: 12px;
}

.test-result.success {
  background: var(--success-light);
  color: var(--success-color);
}

.test-result.error {
  background: var(--danger-bg);
  color: var(--danger-color);
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid var(--separator-color);
}

.btn {
  padding: 8px 20px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn.primary {
  background: var(--accent-color);
  color: white;
  border: none;
}

.btn.primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn.secondary {
  background: var(--control-bg);
  color: var(--text-color);
  border: 1px solid var(--border-color);
}

.btn.secondary:hover:not(:disabled) {
  background: var(--control-hover);
}

.btn.secondary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
