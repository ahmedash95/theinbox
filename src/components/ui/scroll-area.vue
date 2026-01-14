<script setup lang="ts">
import {
  ScrollAreaRoot,
  ScrollAreaViewport,
  ScrollAreaScrollbar,
  ScrollAreaThumb,
} from "radix-vue";
import { computed, useAttrs } from "vue";
import { cn } from "../../lib/utils";

defineOptions({ inheritAttrs: false });

const attrs = useAttrs();
const className = computed(() =>
  cn("relative overflow-hidden", attrs.class as string | undefined)
);
const attrsWithoutClass = computed(() => {
  const { class: _class, ...rest } = attrs as Record<string, unknown>;
  return rest;
});
</script>

<template>
  <ScrollAreaRoot v-bind="attrsWithoutClass" :class="className">
    <ScrollAreaViewport class="h-full w-full rounded-[inherit]">
      <slot />
    </ScrollAreaViewport>
    <ScrollAreaScrollbar
      orientation="vertical"
      class="flex touch-none select-none p-0.5 transition-colors"
    >
      <ScrollAreaThumb class="relative flex-1 rounded-full bg-border" />
    </ScrollAreaScrollbar>
  </ScrollAreaRoot>
</template>
