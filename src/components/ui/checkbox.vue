<script setup lang="ts">
import { CheckboxIndicator, CheckboxRoot } from "radix-vue";
import { Check } from "lucide-vue-next";
import { computed, useAttrs } from "vue";
import { cn } from "../../lib/utils";

defineOptions({ inheritAttrs: false });

const attrs = useAttrs();
const className = computed(() =>
  cn(
    "peer h-4 w-4 shrink-0 rounded-sm border border-primary ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50 data-[state=checked]:bg-primary data-[state=checked]:text-primary-foreground",
    attrs.class as string | undefined
  )
);
const attrsWithoutClass = computed(() => {
  const { class: _class, ...rest } = attrs as Record<string, unknown>;
  return rest;
});
</script>

<template>
  <CheckboxRoot v-bind="attrsWithoutClass" :class="className">
    <CheckboxIndicator class="flex items-center justify-center text-current">
      <Check class="h-3.5 w-3.5" />
    </CheckboxIndicator>
  </CheckboxRoot>
</template>
