<script setup lang="ts">
import { ref, watch, computed } from "vue";
import { X, Trash2, Lightbulb } from "lucide-vue-next";
import type { FilterPattern, FilterField, TestPatternResult, Email } from "../types";
import Button from "./ui/button.vue";
import Input from "./ui/input.vue";
import Checkbox from "./ui/checkbox.vue";
import Badge from "./ui/badge.vue";

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
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-sm"
        @click.self="emit('close')"
      >
        <div class="modal w-full max-w-lg overflow-hidden rounded-lg border bg-card shadow-xl">
          <div class="flex items-center justify-between border-b px-5 py-4">
            <h2 class="text-sm font-semibold">
              {{ editFilter ? "Edit Filter" : "New Filter" }}
            </h2>
            <Button variant="ghost" size="icon" @click="emit('close')" aria-label="Close">
              <X :size="16" />
            </Button>
          </div>

          <div class="space-y-4 px-5 py-4">
            <div class="space-y-2">
              <label class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Name</label>
              <Input
                v-model="name"
                type="text"
                placeholder="e.g., Marketing, Newsletters"
                autofocus
              />
            </div>

            <div class="space-y-2">
              <label class="flex items-center gap-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                Pattern
                <Badge variant="outline">{{ isRegex ? "regex" : "text" }}</Badge>
              </label>
              <Input
                v-model="pattern"
                type="text"
                :placeholder="isRegex ? 'e.g., newsletter|promo' : 'e.g., newsletter'"
                class="font-mono"
              />
              <label class="flex items-center gap-2 text-xs text-muted-foreground">
                <Checkbox v-model:checked="isRegex" />
                Use Regular Expression
              </label>
            </div>

            <div class="space-y-2">
              <label class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Match in</label>
              <div class="flex gap-2">
                <Button
                  size="sm"
                  :variant="field === 'any' ? 'secondary' : 'outline'"
                  @click="field = 'any'"
                >
                  Any
                </Button>
                <Button
                  size="sm"
                  :variant="field === 'subject' ? 'secondary' : 'outline'"
                  @click="field = 'subject'"
                >
                  Subject
                </Button>
                <Button
                  size="sm"
                  :variant="field === 'sender' ? 'secondary' : 'outline'"
                  @click="field = 'sender'"
                >
                  Sender
                </Button>
              </div>
            </div>

            <div
              class="rounded-md border px-4 py-3"
              :class="{
                'bg-muted/40': matchStatus === 'empty',
                'bg-destructive/5 border-destructive/20 text-destructive': matchStatus === 'none',
                'bg-emerald-50 border-emerald-200 text-emerald-700': matchStatus === 'few' || matchStatus === 'some',
                'bg-primary/5 border-primary/20 text-primary': matchStatus === 'many',
              }"
            >
              <div class="flex items-center justify-between text-xs uppercase tracking-wide">
                <span class="font-semibold">Preview</span>
                <span v-if="testing" class="text-xs">Testing...</span>
              </div>

              <div v-if="testError" class="mt-2 text-xs text-destructive">
                {{ testError }}
              </div>

              <div v-else-if="testResult" class="mt-3 space-y-3">
                <div class="flex items-baseline gap-2">
                  <span class="text-2xl font-semibold">{{ testResult.match_count }}</span>
                  <span class="text-xs text-muted-foreground">
                    of {{ testResult.total_count }} emails match
                  </span>
                </div>

                <div v-if="testResult.sample_matches.length > 0" class="space-y-1 border-t pt-3 text-xs">
                  <div
                    v-for="email in testResult.sample_matches"
                    :key="email.id"
                    class="flex items-center gap-2"
                  >
                    <span class="max-w-[120px] truncate font-medium">
                      {{ email.sender.split('<')[0].replace(/"/g, '').trim() }}
                    </span>
                    <span class="truncate text-muted-foreground">{{ email.subject }}</span>
                  </div>
                </div>
              </div>

              <div v-else class="mt-2 text-xs text-muted-foreground">
                Enter a pattern to see matches.
              </div>
            </div>

            <div class="rounded-md border bg-muted/40 px-4 py-3 text-xs text-muted-foreground">
              <div class="flex items-center gap-2 font-semibold text-foreground">
                <Lightbulb :size="14" />
                Tips
              </div>
              <ul class="mt-2 space-y-1">
                <template v-if="!isRegex">
                  <li><code class="rounded bg-background px-1">newsletter</code> — matches anywhere in text</li>
                  <li>Matching is case-insensitive</li>
                </template>
                <template v-else>
                  <li><code class="rounded bg-background px-1">news|promo</code> — matches either word</li>
                  <li><code class="rounded bg-background px-1">facebook.*notify</code> — matches with anything between</li>
                </template>
              </ul>
            </div>
          </div>

          <div class="flex items-center gap-2 border-t px-5 py-4">
            <Button v-if="editFilter" variant="destructive" size="sm" @click="deleteFilter">
              <Trash2 :size="14" />
              Delete
            </Button>
            <div class="ml-auto flex gap-2">
              <Button variant="outline" @click="emit('close')">Cancel</Button>
              <Button :disabled="!canSave" @click="save">
                {{ editFilter ? "Save" : "Create" }}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.18s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active .modal,
.modal-leave-active .modal {
  transition: transform 0.18s ease, opacity 0.18s ease;
}

.modal-enter-from .modal,
.modal-leave-to .modal {
  transform: scale(0.98) translateY(-6px);
  opacity: 0;
}
</style>
