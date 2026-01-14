<script setup lang="ts">
import { Separator as SeparatorRoot } from "radix-vue";
import { computed, useAttrs } from "vue";
import { cn } from "../../lib/utils";

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    orientation?: "horizontal" | "vertical";
    decorative?: boolean;
  }>(),
  {
    orientation: "horizontal",
    decorative: true,
  }
);

const attrs = useAttrs();
const className = computed(() =>
  cn(
    "shrink-0 bg-border",
    props.orientation === "vertical" ? "h-full w-px" : "h-px w-full",
    attrs.class as string | undefined
  )
);
const attrsWithoutClass = computed(() => {
  const { class: _class, ...rest } = attrs as Record<string, unknown>;
  return rest;
});
</script>

<template>
  <SeparatorRoot
    v-bind="attrsWithoutClass"
    :class="className"
    :orientation="props.orientation"
    :decorative="props.decorative"
  />
</template>
