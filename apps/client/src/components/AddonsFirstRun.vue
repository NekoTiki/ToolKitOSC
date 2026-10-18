<script setup lang="ts">
import type { Addon } from '@renderer/composables/useAddons'
import { useAddons } from '@renderer/composables/useAddons'
import { computed, ref } from 'vue'

// A new install asks once which add-ons you use, as big tiles you tap to turn on. Skip leaves
// everything off. Either way it doesn't come back: Settings > Add-ons changes it later.
const { addons, setEnabled, finishSetup } = useAddons()
const toast = useToast()

const picked = ref(new Set<string>())
const saving = ref(false)

const toggle = (addon: Addon): void => {
  const next = new Set(picked.value)

  if (next.has(addon.id)) next.delete(addon.id)
  else next.add(addon.id)
  picked.value = next
}

// What gets turned on: the picks and whatever they need, needs first.
const toTurnOn = computed(() => {
  const order: string[] = []
  const add = (id: string): void => {
    if (order.includes(id)) return
    const addon = addons.value.find((a) => a.id === id)

    addon?.needs.forEach(add)
    // Needs one of several: the first, unless one of them is already picked.
    if (addon?.needsAny.length && !addon.needsAny.some((need) => picked.value.has(need) || order.includes(need))) add(addon.needsAny[0]!)
    order.push(id)
  }

  addons.value.filter((addon) => picked.value.has(addon.id)).forEach((addon) => add(addon.id))

  return order
})

const done = async (turnOn: string[]): Promise<void> => {
  saving.value = true
  try {
    for (const id of turnOn) await setEnabled(id, true)
    await finishSetup(false)
  } catch (error) {
    toast.add({ title: 'Could not turn on your add-ons', description: String(error), color: 'error' })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="grid min-h-0 overflow-auto p-6">
    <div class="m-auto grid w-full max-w-3xl gap-5">
      <div class="grid gap-1.5">
        <h1 class="text-3xl font-semibold text-highlighted">
          What do you use?
        </h1>
        <p class="text-muted">
          Turn on what you own. The rest stays off: no connections, no extra pages. You can change this any time in Settings › Add-ons.
        </p>
      </div>

      <div class="grid grid-cols-2 gap-3 max-md:grid-cols-1">
        <button
          v-for="addon in addons"
          :key="addon.id"
          type="button"
          class="grid min-h-32 cursor-pointer content-start gap-2 rounded-panel border p-5 text-left transition-colors"
          :class="picked.has(addon.id) ? 'border-primary/60 bg-primary/14' : 'border-transparent glass hover:bg-elevated/60'"
          :aria-pressed="picked.has(addon.id)"
          @click="toggle(addon)"
        >
          <span class="flex items-center justify-between gap-3">
            <b class="text-lg font-semibold text-highlighted">{{ addon.name }}</b>
            <UIcon
              :name="picked.has(addon.id) ? 'i-lucide-circle-check' : 'i-lucide-circle'"
              class="size-6"
              :class="picked.has(addon.id) ? 'text-primary' : 'text-muted'"
            />
          </span>
          <span class="text-sm text-muted">{{ addon.pitch }}</span>
          <UBadge
            v-if="addon.badges.includes('shocks') || addon.badges.includes('electrical')"
            :label="addon.badges.includes('shocks') ? 'Shocks' : 'Electrical'"
            color="error"
            variant="subtle"
            class="w-fit rounded-full"
          />
        </button>
      </div>

      <div class="flex flex-wrap justify-end gap-2.5">
        <UButton
          color="neutral"
          variant="subtle"
          size="xl"
          class="rounded-2xl"
          :disabled="saving"
          @click="done([])"
        >
          Skip
        </UButton>
        <UButton
          size="xl"
          class="rounded-2xl"
          icon="i-lucide-check"
          :loading="saving"
          @click="done(toTurnOn)"
        >
          {{ picked.size ? `Turn on ${picked.size} and continue` : 'Continue' }}
        </UButton>
      </div>
    </div>
  </div>
</template>
