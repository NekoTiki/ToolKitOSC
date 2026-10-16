<script setup lang="ts">
import EmptyState from '@renderer/components/ui/EmptyState.vue'
import FormSection from '@renderer/components/ui/FormSection.vue'
import type { HapticsToy } from '@renderer/composables/useAvatarHaptics'
import { SOURCE_CONTACTS, SOURCE_ZONE_KIND, useAvatarHaptics, ZONE_KIND_LABEL } from '@renderer/composables/useAvatarHaptics'
import type { HapticsSource } from '@renderer/lib/tauri-bridge'
import { computed } from 'vue'

// One haptics source of a toy, as a sub-page of the toy on the Haptics page: which zones it reads,
// what it reacts to, and how its value is shaped. Changes apply right away, so you can feel them.
const props = defineProps<{ toy: HapticsToy; index: number }>()

const { zones, editedOutput, activeOutput, level, change, outputToEdit } = useAvatarHaptics()

const toyKey = computed(() => props.toy.key)
const output = computed(() => editedOutput(toyKey.value))
const source = computed(() => output.value?.sources[props.index])

const kind = computed(() => (source.value ? SOURCE_ZONE_KIND[source.value.kind] : 'socket'))
const kindLabel = computed(() => ZONE_KIND_LABEL[kind.value])
const live = computed(() => !!output.value && output.value === activeOutput(toyKey.value))
const pct = (value: number): string => `${Math.round(value * 100)} %`

const edit = (mutate: (source: HapticsSource) => void): void => {
  const current = props.toy
  change((target) => {
    const found = outputToEdit(target, current).sources[props.index]

    if (found) mutate(found)
  })
}

// Zone chips: the worn avatar's zones of this kind, plus any the source names that it doesn't have.
const zoneNames = computed(() => {
  const names = zones.value.filter((zone) => zone.kind === kind.value).map((zone) => zone.name)

  for (const name of source.value?.zones.include ?? []) if (name.trim() && !names.includes(name)) names.push(name)

  return names
})
const zoneLevel = (name: string): number => zones.value.find((zone) => zone.kind === kind.value && zone.name === name)?.level ?? 0
const everyZone = computed(() => !source.value?.zones.include.some((name) => name.trim()))

const toggleZone = (name: string): void =>
  edit((target) => {
    const included = target.zones.include.includes(name)
    target.zones.include = included ? target.zones.include.filter((item) => item !== name) : [...target.zones.include, name]
  })

const contactOn = (key: string): boolean => !!(source.value as unknown as Record<string, boolean> | undefined)?.[key]
const setContact = (key: string, on: boolean): void => edit((target) => ((target as unknown as Record<string, boolean>)[key] = on))
</script>

<template>
  <div
    v-if="source"
    class="grid max-w-3xl gap-3.5"
  >
    <FormSection
      title="Zones"
      :description="`Which ${kindLabel.toLowerCase()}s of your avatar it reads, by name. None picked means every one.`"
    >
      <div class="flex flex-wrap gap-1.5">
        <button
          type="button"
          class="min-h-10 cursor-pointer rounded-full border px-3.5 text-sm transition-colors"
          :class="everyZone ? 'border-primary/50 bg-primary/15 text-highlighted' : 'border-transparent bg-(--aurora-well) text-muted hover:text-default'"
          :aria-pressed="everyZone"
          @click="edit((target) => (target.zones.include = []))"
        >
          Every {{ kindLabel.toLowerCase() }}
        </button>
        <button
          v-for="name in zoneNames"
          :key="name"
          type="button"
          class="min-h-10 cursor-pointer rounded-full border px-3.5 text-sm transition-colors"
          :class="
            source.zones.include.includes(name)
              ? 'border-primary/50 bg-primary/15 text-highlighted'
              : 'border-transparent bg-(--aurora-well) text-muted hover:text-default'
          "
          :aria-pressed="source.zones.include.includes(name)"
          @click="toggleZone(name)"
        >
          {{ name }}
          <span
            v-if="zoneLevel(name) > 0"
            class="text-secondary tabular-nums"
          > · {{ zoneLevel(name).toFixed(2) }}</span>
        </button>
      </div>
      <p
        v-if="!zoneNames.length"
        class="text-sm text-muted"
      >
        The avatar you wear has no {{ kindLabel.toLowerCase() }}s. This source reads every one on avatars that do.
      </p>
    </FormSection>

    <FormSection
      title="Reacts to"
      :description="live ? `Now: ${pct(level(toyKey).sources[index] ?? 0)}` : undefined"
    >
      <div class="grid gap-2 sm:grid-cols-2">
        <div
          v-for="contact in SOURCE_CONTACTS[source.kind]"
          :key="contact.key"
          class="flex min-h-12 items-center gap-3 rounded-field bg-(--aurora-well) px-3.5 py-2"
        >
          <b class="flex-1 text-sm font-semibold text-highlighted">{{ contact.label }}</b>
          <USwitch
            :model-value="contactOn(contact.key)"
            :aria-label="contact.label"
            @update:model-value="setContact(contact.key, $event)"
          />
        </div>
        <div class="flex min-h-12 items-center gap-3 rounded-field bg-(--aurora-well) px-3.5 py-2">
          <span class="grid flex-1">
            <b class="text-sm font-semibold text-highlighted">Follow movement</b>
            <small class="text-xs text-muted">Speed instead of depth</small>
          </span>
          <USwitch
            :model-value="source.shape.followMovement"
            aria-label="Follow movement"
            @update:model-value="edit((target) => (target.shape.followMovement = $event))"
          />
        </div>
      </div>
    </FormSection>

    <div class="grid gap-3.5 sm:grid-cols-2">
      <FormSection
        title="Dead zone"
        description="Values under it read 0, so a hand nearby doesn't buzz the toy."
      >
        <div class="grid gap-2">
          <div class="flex justify-end text-sm">
            <b class="font-semibold text-highlighted tabular-nums">{{ pct(source.shape.deadZone) }}</b>
          </div>
          <USlider
            :model-value="Math.round(source.shape.deadZone * 100)"
            :min="0"
            :max="90"
            :step="1"
            size="lg"
            aria-label="Dead zone"
            @update:model-value="edit((target) => (target.shape.deadZone = ($event ?? 0) / 100))"
          />
        </div>
      </FormSection>
      <FormSection
        title="Max strength"
        description="The most this source drives the toy."
      >
        <div class="grid gap-2">
          <div class="flex justify-end text-sm">
            <b class="font-semibold text-highlighted tabular-nums">{{ pct(source.shape.scale) }}</b>
          </div>
          <USlider
            :model-value="Math.round(source.shape.scale * 100)"
            :min="0"
            :max="100"
            :step="1"
            size="lg"
            aria-label="Max strength"
            @update:model-value="edit((target) => (target.shape.scale = ($event ?? 0) / 100))"
          />
        </div>
      </FormSection>
    </div>
  </div>

  <EmptyState
    v-else
    icon="i-lucide-hand-heart"
    title="Source not found"
    description="It may have been deleted."
  />
</template>
