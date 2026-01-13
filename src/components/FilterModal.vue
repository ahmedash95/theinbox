<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { X, Trash2, Lightbulb } from "lucide-vue-next";
import type { FilterPattern, FilterField, TestPatternResult, Email } from "../types";

const props = defineProps<{
  show: boolean;
  editFilter?: FilterPattern | null;
  emails: Email[];
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save", filter: FilterPattern): void;
  (e: "delete", filterId: string): void;
}>();

const name = ref("");
const pattern = ref("");
const field = ref<FilterField>("any");
const isRegex = ref(false);

const testResult = ref<TestPatternResult | null>(null);
const testing = ref(false);
const testError = ref<string | null>(null);

// Reset form when modal opens
watch(
  () => props.show,
  (show) => {
    if (show) {
      if (props.editFilter) {
        name.value = props.editFilter.name;
        pattern.value = props.editFilter.pattern;
        field.value = props.editFilter.field;
        isRegex.value = props.editFilter.is_regex;
      } else {
        name.value = "";
        pattern.value = "";
        field.value = "any";
        isRegex.value = false;
      }
      testResult.value = null;
      testError.value = null;
    }
  }
);

// Auto-test pattern as user types (debounced)
let testTimeout: ReturnType<typeof setTimeout> | null = null;
watch([pattern, field, isRegex], () => {
  if (testTimeout) clearTimeout(testTimeout);
  testError.value = null;
  
  if (pattern.value.trim()) {
    testTimeout = setTimeout(() => testPattern(), 300);
  } else {
    testResult.value = null;
  }
});

// Test pattern locally against passed emails
function testPattern() {
  if (!pattern.value.trim()) return;

  testing.value = true;
  testError.value = null;

  try {
    const patternStr = pattern.value;
    const fieldVal = field.value;
    const isRegexVal = isRegex.value;
    
    let matched: Email[];
    
    if (isRegexVal) {
      // Regex matching
      const regex = new RegExp(patternStr, "i");
      matched = props.emails.filter((email) => {
        switch (fieldVal) {
          case "subject":
            return regex.test(email.subject);
          case "sender":
            return regex.test(email.sender);
          case "any":
            return regex.test(email.subject) || regex.test(email.sender);
          default:
            return false;
        }
      });
    } else {
      // Simple case-insensitive substring match
      const patternLower = patternStr.toLowerCase();
      matched = props.emails.filter((email) => {
        switch (fieldVal) {
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
      });
    }
    
    testResult.value = {
      match_count: matched.length,
      total_count: props.emails.length,
      sample_matches: matched.slice(0, 5),
    };
  } catch (e) {
    testError.value = String(e);
    testResult.value = null;
  } finally {
    testing.value = false;
  }
}

function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substr(2);
}

function save() {
  if (!name.value.trim() || !pattern.value.trim()) return;

  const filter: FilterPattern = {
    id: props.editFilter?.id || generateId(),
    name: name.value.trim(),
    pattern: pattern.value.trim(),
    field: field.value,
    is_regex: isRegex.value,
    enabled: props.editFilter?.enabled ?? true,
  };

  emit("save", filter);
  emit("close");
}

function deleteFilter() {
  if (props.editFilter) {
    emit("delete", props.editFilter.id);
  }
}

const canSave = computed(() => {
  return name.value.trim() && pattern.value.trim() && !testError.value;
});

const matchStatus = computed(() => {
  if (!testResult.value) return "empty";
  if (testResult.value.match_count === 0) return "none";
  if (testResult.value.match_count <= 5) return "few";
  if (testResult.value.match_count <= 20) return "some";
  return "many";
});
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="show" class="modal-overlay" @click.self="emit('close')">
        <div class="modal">
          <!-- Header -->
          <div class="modal-header">
            <h2>{{ editFilter ? "Edit Filter" : "New Filter" }}</h2>
            <button class="close-btn" @click="emit('close')">
              <X :size="18" />
            </button>
          </div>

          <!-- Body -->
          <div class="modal-body">
            <!-- Name -->
            <div class="form-group">
              <label>Name</label>
              <input
                v-model="name"
                type="text"
                placeholder="e.g., Marketing, Newsletters"
                class="input"
                autofocus
              />
            </div>

            <!-- Pattern -->
            <div class="form-group">
              <label>
                Pattern
                <span class="label-badge" :class="{ active: isRegex }">
                  {{ isRegex ? "regex" : "text" }}
                </span>
              </label>
              <input
                v-model="pattern"
                type="text"
                :placeholder="isRegex ? 'e.g., newsletter|promo' : 'e.g., newsletter'"
                class="input mono"
              />
              <label class="checkbox-label">
                <input type="checkbox" v-model="isRegex" />
                <span>Use Regular Expression</span>
              </label>
            </div>

            <!-- Field selector -->
            <div class="form-group">
              <label>Match in</label>
              <div class="segment-control">
                <button
                  :class="{ active: field === 'any' }"
                  @click="field = 'any'"
                >
                  Any
                </button>
                <button
                  :class="{ active: field === 'subject' }"
                  @click="field = 'subject'"
                >
                  Subject
                </button>
                <button
                  :class="{ active: field === 'sender' }"
                  @click="field = 'sender'"
                >
                  Sender
                </button>
              </div>
            </div>

            <!-- Live Preview -->
            <div class="preview-card" :class="matchStatus">
              <div class="preview-header">
                <span class="preview-label">Preview</span>
                <span v-if="testing" class="preview-status">Testing...</span>
              </div>

              <div v-if="testError" class="preview-error">
                {{ testError }}
              </div>

              <div v-else-if="testResult" class="preview-content">
                <div class="match-stat">
                  <span class="match-number">{{ testResult.match_count }}</span>
                  <span class="match-label">of {{ testResult.total_count }} emails match</span>
                </div>

                <div v-if="testResult.sample_matches.length > 0" class="sample-list">
                  <div
                    v-for="email in testResult.sample_matches"
                    :key="email.id"
                    class="sample-item"
                  >
                    <span class="sample-sender">{{ email.sender.split('<')[0].replace(/"/g, '').trim() }}</span>
                    <span class="sample-subject">{{ email.subject }}</span>
                  </div>
                </div>
              </div>

              <div v-else class="preview-empty">
                Enter a pattern to see matches
              </div>
            </div>

            <!-- Tips -->
            <div class="tips-card">
              <div class="tips-header">
                <Lightbulb :size="14" />
                <span>Tips</span>
              </div>
              <ul>
                <template v-if="!isRegex">
                  <li><code>newsletter</code> — matches anywhere in text</li>
                  <li>Matching is case-insensitive</li>
                </template>
                <template v-else>
                  <li><code>news|promo</code> — matches either word</li>
                  <li><code>facebook.*notify</code> — matches with anything between</li>
                </template>
              </ul>
            </div>

          </div>

          <!-- Footer -->
          <div class="modal-footer">
            <button 
              v-if="editFilter" 
              class="btn danger" 
              @click="deleteFilter"
            >
              <Trash2 :size="14" />
              Delete
            </button>
            <div class="footer-spacer"></div>
            <button class="btn secondary" @click="emit('close')">Cancel</button>
            <button class="btn primary" @click="save" :disabled="!canSave">
              {{ editFilter ? "Save" : "Create" }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
/* Modal transition */
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.2s ease;
}

.modal-enter-active .modal,
.modal-leave-active .modal {
  transition: transform 0.2s ease, opacity 0.2s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-from .modal,
.modal-leave-to .modal {
  transform: scale(0.95) translateY(-10px);
  opacity: 0;
}

/* Overlay */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(4px);
  -webkit-backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

/* Modal */
.modal {
  width: 100%;
  max-width: 420px;
  max-height: 90vh;
  border-radius: 12px;
  background: var(--surface-primary);
  backdrop-filter: blur(40px) saturate(180%);
  -webkit-backdrop-filter: blur(40px) saturate(180%);
  border: 1px solid var(--border-color);
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.25);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* Header */
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px;
  border-bottom: 1px solid var(--separator-color);
}

.modal-header h2 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}

.close-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 6px;
  background: var(--control-bg);
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.15s;
}

.close-btn:hover {
  background: var(--control-hover);
  color: var(--text-color);
}

/* Body */
.modal-body {
  padding: 16px;
  overflow-y: auto;
  flex: 1;
}

.form-group {
  margin-bottom: 16px;
}

.form-group label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.label-badge {
  font-size: 9px;
  font-weight: 600;
  text-transform: uppercase;
  padding: 2px 5px;
  border-radius: 4px;
  background: var(--control-bg);
  color: var(--text-tertiary);
}

.label-badge.active {
  background: var(--accent-light);
  color: var(--accent-color);
}

.input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  font-size: 13px;
  background: var(--surface-secondary);
  color: var(--text-color);
  transition: all 0.15s;
}

.input:focus {
  outline: none;
  border-color: var(--accent-color);
  background: var(--surface-primary);
  box-shadow: 0 0 0 3px var(--accent-light);
}

.input.mono {
  font-family: "SF Mono", Monaco, Consolas, monospace;
}

.checkbox-label {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
  cursor: pointer;
  font-size: 12px;
  color: var(--text-secondary);
}

.checkbox-label input {
  width: 14px;
  height: 14px;
  accent-color: var(--accent-color);
}

/* Segment control */
.segment-control {
  display: flex;
  background: var(--control-bg);
  border-radius: 8px;
  padding: 2px;
}

.segment-control button {
  flex: 1;
  padding: 7px 12px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.segment-control button:hover:not(.active) {
  color: var(--text-color);
}

.segment-control button.active {
  background: var(--control-active);
  color: var(--text-color);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
}

/* Preview card */
.preview-card {
  background: var(--control-bg);
  border-radius: 10px;
  padding: 12px;
  margin-bottom: 12px;
}

.preview-card.none {
  background: var(--danger-bg);
}

.preview-card.few,
.preview-card.some {
  background: var(--success-light);
}

.preview-card.many {
  background: var(--accent-light);
}

.preview-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.preview-label {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-tertiary);
}

.preview-status {
  font-size: 11px;
  color: var(--accent-color);
}

.preview-error {
  font-size: 12px;
  color: var(--danger-color);
}

.preview-content {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.match-stat {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.match-number {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-color);
  line-height: 1;
}

.match-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.sample-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding-top: 8px;
  border-top: 1px solid var(--border-color);
}

.sample-item {
  display: flex;
  gap: 8px;
  font-size: 11px;
  overflow: hidden;
}

.sample-sender {
  font-weight: 500;
  color: var(--text-color);
  white-space: nowrap;
  flex-shrink: 0;
  max-width: 100px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sample-subject {
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.preview-empty {
  font-size: 12px;
  color: var(--text-tertiary);
  text-align: center;
  padding: 8px;
}

/* Tips */
.tips-card {
  background: var(--control-bg);
  border-radius: 10px;
  padding: 12px;
  font-size: 11px;
  margin-bottom: 12px;
}

.tips-header {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 600;
  margin-bottom: 8px;
  color: var(--text-secondary);
}

.tips-card ul {
  margin: 0;
  padding-left: 16px;
  color: var(--text-secondary);
}

.tips-card li {
  margin-bottom: 4px;
}

.tips-card code {
  background: var(--surface-secondary);
  padding: 1px 5px;
  border-radius: 4px;
  font-family: "SF Mono", Monaco, Consolas, monospace;
  font-size: 10px;
}

/* Footer */
.modal-footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--separator-color);
}

.footer-spacer {
  flex: 1;
}

.btn {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.btn.secondary {
  background: var(--control-bg);
  color: var(--text-color);
}

.btn.secondary:hover {
  background: var(--control-hover);
}

.btn.primary {
  background: var(--accent-color);
  color: white;
}

.btn.primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.btn.primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn.danger {
  display: flex;
  align-items: center;
  gap: 6px;
  background: var(--danger-bg);
  color: var(--danger-color);
}

.btn.danger:hover {
  background: var(--danger-color);
  color: white;
}
</style>
