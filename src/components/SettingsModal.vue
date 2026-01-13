<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { EmailProvider } from "../types";

const props = defineProps<{
  show: boolean;
  currentProvider: EmailProvider | null;
  currentGmailEmail: string | null;
}>();

const emit = defineEmits<{
  close: [];
  save: [provider: EmailProvider, gmailEmail?: string];
}>();

const selectedProvider = ref<EmailProvider | null>(props.currentProvider);
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

const canSave = computed(() => {
  if (!selectedProvider.value) return false;
  if (selectedProvider.value === "apple_mail") return true;
  if (selectedProvider.value === "gmail") {
    // Can save if already configured or has new password
    return (
      gmailEmail.value.includes("@") &&
      (isConfigured.value || gmailAppPassword.value.length >= 16)
    );
  }
  return false;
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
  if (!canSave.value || !selectedProvider.value) return;

  saving.value = true;

  try {
    if (selectedProvider.value === "gmail" && gmailAppPassword.value) {
      // Store credentials in keychain
      await invoke("gmail_store_credentials", {
        email: gmailEmail.value,
        appPassword: gmailAppPassword.value,
      });
    }

    emit(
      "save",
      selectedProvider.value,
      selectedProvider.value === "gmail" ? gmailEmail.value : undefined
    );
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
          <!-- Provider Selection -->
          <div class="section">
            <label class="section-label">Email Provider</label>
            <div class="provider-cards">
              <button
                class="provider-card"
                :class="{ active: selectedProvider === 'apple_mail' }"
                @click="selectedProvider = 'apple_mail'"
              >
                <div class="provider-icon">📮</div>
                <div class="provider-name">Apple Mail</div>
                <div class="provider-desc">Local mailboxes via Mail.app</div>
              </button>

              <button
                class="provider-card"
                :class="{ active: selectedProvider === 'gmail' }"
                @click="selectedProvider = 'gmail'"
              >
                <div class="provider-icon">📧</div>
                <div class="provider-name">Gmail</div>
                <div class="provider-desc">Direct IMAP connection</div>
              </button>
            </div>
          </div>

          <!-- Gmail Configuration -->
          <div v-if="selectedProvider === 'gmail'" class="section">
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

          <!-- Apple Mail Info -->
          <div v-if="selectedProvider === 'apple_mail'" class="section">
            <div class="info-box">
              <strong>Requirements:</strong>
              <ul>
                <li>Apple Mail must be set up with your accounts</li>
                <li>For best performance, grant Full Disk Access in System Settings → Privacy</li>
              </ul>
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

.provider-cards {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}

.provider-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px 16px;
  border: 2px solid var(--border-color);
  border-radius: 12px;
  background: var(--surface-secondary);
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: center;
}

.provider-card:hover {
  border-color: var(--text-tertiary);
  background: var(--surface-hover);
}

.provider-card.active {
  border-color: var(--accent-color);
  background: var(--accent-light);
}

.provider-icon {
  font-size: 32px;
  margin-bottom: 8px;
}

.provider-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-color);
  margin-bottom: 4px;
}

.provider-desc {
  font-size: 11px;
  color: var(--text-tertiary);
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

.info-box {
  padding: 16px;
  background: var(--surface-tertiary);
  border-radius: 8px;
  font-size: 13px;
  color: var(--text-secondary);
}

.info-box strong {
  display: block;
  margin-bottom: 8px;
  color: var(--text-color);
}

.info-box ul {
  margin: 0;
  padding-left: 20px;
}

.info-box li {
  margin-bottom: 4px;
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
