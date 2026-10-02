<script setup lang="ts">
import { CONTROL_ICONS } from '@vrc-osc-toolkit/shared-ui'
import { computed, ref, watch } from 'vue'

// An icon picker: a field-like button showing the current icon, opening a searchable grid of
// large icon buttons grouped by category - recognising an icon is quicker than reading its name in
// a list, and the big targets are easy to hit from VR. Icons come from the single shared list (see
// packages/shared-ui's constants/controlIcons.ts) - the AI control-suggestion feature reads the
// exact same list server side, so a control it picks an icon for always matches one offered here.
//
// Attributes (class, @keyup.backspace to clear...) land on the trigger button, so it sizes and
// behaves like the input it replaces.
defineOptions({ inheritAttrs: false })

const model = defineModel<string>()

const open = ref(false)
const query = ref('')

watch(open, (value) => {
  if (value) query.value = ''
})

const selected = computed(() => CONTROL_ICONS.find((entry) => entry.icon === model.value))

const categories = computed(() => {
  const search = query.value.trim().toLowerCase()
  const groups = new Map<string, typeof CONTROL_ICONS>()

  for (const entry of CONTROL_ICONS) {
    if (search && !`${entry.label} ${entry.category} ${entry.icon}`.toLowerCase().includes(search)) continue

    groups.set(entry.category, [...(groups.get(entry.category) ?? []), entry])
  }

  return [...groups].map(([name, icons]) => ({ name, icons }))
})

const pick = (icon: string): void => {
  model.value = icon
  open.value = false
}
</script>

<template>
  <UPopover
    v-model:open="open"
    :content="{ align: 'start', side: 'bottom', sideOffset: 6 }"
    :ui="{ content: 'w-88 max-w-[calc(100vw-2rem)] rounded-panel p-3' }"
  >
    <UButton
      v-bind="$attrs"
      color="neutral"
      variant="outline"
      :leading-icon="model || 'i-lucide-image'"
      trailing-icon="i-lucide-chevron-down"
      :ui="{ base: 'justify-start', leadingIcon: model ? 'text-highlighted' : 'text-dimmed', trailingIcon: 'ms-auto text-dimmed' }"
      :aria-label="selected ? `Icon: ${selected.label}` : 'Select an icon'"
    >
      <span
        class="truncate"
        :class="model ? 'text-highlighted' : 'text-dimmed'"
      >{{ selected?.label ?? (model ? 'Custom icon' : 'Select an icon') }}</span>
    </UButton>

    <template #content>
      <div class="grid gap-3">
        <div class="flex gap-2">
          <UInput
            v-model="query"
            icon="i-lucide-search"
            placeholder="Search icons"
            autofocus
            class="min-w-0 flex-1"
            :ui="{ base: 'rounded-2xl' }"
          />
          <UButton
            v-if="model"
            icon="i-lucide-circle-x"
            color="neutral"
            variant="subtle"
            @click="pick('')"
          >
            None
          </UButton>
        </div>

        <div class="-mr-1.5 grid max-h-80 gap-3 overflow-y-auto pr-1.5">
          <section
            v-for="category in categories"
            :key="category.name"
            class="grid gap-1.5"
          >
            <h3 class="px-0.5 font-mono text-[10.5px] tracking-widest text-muted uppercase">
              {{ category.name }}
            </h3>
            <div class="grid grid-cols-[repeat(auto-fill,minmax(48px,1fr))] gap-1.5">
              <UTooltip
                v-for="entry in category.icons"
                :key="entry.icon"
                :text="entry.label"
                :delay-duration="300"
              >
                <button
                  type="button"
                  class="grid h-12 cursor-pointer place-items-center rounded-xl transition-colors"
                  :class="entry.icon === model ? 'bg-primary text-inverted' : 'bg-(--aurora-well) text-muted hover:bg-accented hover:text-highlighted'"
                  :aria-label="entry.label"
                  :aria-pressed="entry.icon === model"
                  @click="pick(entry.icon)"
                >
                  <UIcon
                    :name="entry.icon"
                    class="size-5"
                  />
                </button>
              </UTooltip>
            </div>
          </section>

          <p
            v-if="!categories.length"
            class="py-6 text-center text-sm text-muted"
          >
            No icons match "{{ query.trim() }}".
          </p>
        </div>
      </div>
    </template>
  </UPopover>
</template>
