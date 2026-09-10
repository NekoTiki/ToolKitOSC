<script setup lang="ts">
import type { IntifaceDevice } from '@renderer/composables/useIntiface'
import type { IntifaceToyControl } from '@vrc-osc-toolkit/shared-ui'
import { computed } from 'vue'

const props = defineProps<{ devices: Map<number, IntifaceDevice> }>()

const actuators = defineModel<IntifaceToyControl['actuators']>('actuators', { required: true })

const deviceList = computed(() => Array.from(props.devices.values()))

const isSelected = (deviceIndex: number, actuatorIndex: number): boolean =>
  actuators.value.some((a) => a.deviceIndex === deviceIndex && a.actuatorIndex === actuatorIndex)

// Toys are listed one per group with a checkbox per actuator (vibrate, rotate, oscillate, ...) -
// a toy with a single motor is effectively "select this toy", a toy with several asks about each
// one independently, per the task's requirement, rather than only letting the whole toy be
// selected as one unit.
const toggle = (
  device: IntifaceDevice,
  actuator: IntifaceDevice['actuators'][number],
  checked: boolean
): void => {
  if (checked) {
    if (isSelected(device.index, actuator.index)) return

    actuators.value = [
      ...actuators.value,
      {
        deviceIndex: device.index,
        deviceName: device.name,
        actuatorIndex: actuator.index,
        actuatorDescription: actuator.description,
        actuatorType: actuator.actuatorType
      }
    ]
  } else {
    actuators.value = actuators.value.filter(
      (a) => !(a.deviceIndex === device.index && a.actuatorIndex === actuator.index)
    )
  }
}
</script>

<template>
  <UFormField
    label="Toys"
    required
    description="Pick which actuators this slider controls. Toys with more than one motor (vibrate, rotate, oscillate, ...) list each one separately."
  >
    <div
      v-if="deviceList.length === 0"
      class="text-sm text-muted"
    >
      No toys connected. Make sure Intiface Central is running and a toy is paired.
    </div>
    <div
      v-else
      class="flex flex-col gap-3"
    >
      <div
        v-for="device in deviceList"
        :key="device.index"
        class="flex flex-col gap-1.5 rounded-lg border border-default p-3"
      >
        <p class="text-sm font-medium">
          {{ device.name }}
        </p>
        <!-- Explicit, per-checkbox id: without one, Nuxt UI's useFormField() has every checkbox
        here inherit the *same* id from the single enclosing UFormField, so their <label for>
        all point at the first checkbox's input - clicking any row's label text then toggles
        only the first actuator, not the one actually clicked. -->
        <UCheckbox
          v-for="actuator in device.actuators"
          :id="`intiface-actuator-${device.index}-${actuator.index}`"
          :key="actuator.index"
          :model-value="isSelected(device.index, actuator.index)"
          :label="actuator.description"
          @update:model-value="toggle(device, actuator, !!$event)"
        />
      </div>
    </div>
  </UFormField>
</template>

<style scoped></style>
