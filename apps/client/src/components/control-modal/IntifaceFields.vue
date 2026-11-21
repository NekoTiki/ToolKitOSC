<script setup lang="ts">
import CheckChip from '@renderer/components/ui/CheckChip.vue'
import type { IntifaceDevice } from '@renderer/composables/useIntiface'
import { THIS_PC, toyKey, useIntiface } from '@renderer/composables/useIntiface'
import { api } from '@renderer/lib/tauri-bridge'
import type { IntifaceToyControl } from '@toolkitosc/shared-ui'
import { computed, onBeforeUnmount, ref } from 'vue'

const props = defineProps<{
  // Connected toys, by toyKey().
  devices: Map<string, IntifaceDevice>
  knownDevices: IntifaceDevice[]
  // For pickers that aren't a toy control's (automation steps).
  required?: boolean
  description?: string
}>()

const actuators = defineModel<IntifaceToyControl['actuators']>('actuators', { required: true })

// With several Intiface servers, each toy says which one it's on.
const { toyServerName } = useIntiface()

// Connected toys, then the ones that are off: every toy Intiface has listed before, and the toys
// this control already picked (it may predate the remembered list). By name within each.
const deviceList = computed(() => {
  const offline = new Map<string, IntifaceDevice>()
  for (const device of props.knownDevices) {
    const key = toyKey(device.server, device.index)
    if (!props.devices.has(key)) offline.set(key, { ...device })
  }
  for (const actuator of actuators.value) {
    const key = toyKey(actuator.server, actuator.deviceIndex)
    if (props.devices.has(key)) continue
    const device = offline.get(key) ?? {
      server: actuator.server ?? THIS_PC,
      index: actuator.deviceIndex,
      name: actuator.deviceName,
      actuators: [],
      batterySensorIndex: null,
      battery: null
    }
    if (!device.actuators.some((a) => a.index === actuator.actuatorIndex)) {
      device.actuators = [
        ...device.actuators,
        { index: actuator.actuatorIndex, description: actuator.actuatorDescription, actuatorType: actuator.actuatorType }
      ].sort((a, b) => a.index - b.index)
    }
    offline.set(key, device)
  }
  const byName = (a: IntifaceDevice, b: IntifaceDevice): number => a.name.localeCompare(b.name) || a.server.localeCompare(b.server) || a.index - b.index
  return [
    ...Array.from(props.devices.values()).sort(byName).map((device) => ({ device, online: true })),
    ...Array.from(offline.values()).sort(byName).map((device) => ({ device, online: false }))
  ]
})

// Whether this ref is that motor. A ref saved before there were several servers is on this PC.
const isMotor = (ref: { server?: string; deviceIndex: number; actuatorIndex: number }, device: IntifaceDevice, actuatorIndex: number): boolean =>
  (ref.server ?? THIS_PC) === device.server && ref.deviceIndex === device.index && ref.actuatorIndex === actuatorIndex

const isSelected = (device: IntifaceDevice, actuatorIndex: number): boolean => actuators.value.some((a) => isMotor(a, device, actuatorIndex))

// Unique per toy across servers, for keys, ids and the test button.
const motorKey = (device: IntifaceDevice, actuatorIndex: number): string => `${toyKey(device.server, device.index)}-${actuatorIndex}`

// Toys are listed one per group with a chip per actuator (vibrate, rotate, oscillate, ...): a toy
// with a single motor is effectively "select this toy", a toy with several asks about each one.
// Motors of the same kind are numbered (Vibrate 1, Vibrate 2), since Intiface often names them alike.
const actuatorLabel = (device: IntifaceDevice, actuator: IntifaceDevice['actuators'][number]): string => {
  const same = device.actuators.filter((a) => a.description === actuator.description)

  return same.length > 1 ? `${actuator.description} ${same.indexOf(actuator) + 1}` : actuator.description
}

// Test: runs one motor gently for a second, to find out which one it is.
const TEST_LEVEL = 0.5
const TEST_MS = 1000
const testing = ref<string | null>(null)
let testTimer: ReturnType<typeof setTimeout> | undefined
let testStop: (() => void) | undefined

const test = (device: IntifaceDevice, actuator: IntifaceDevice['actuators'][number]): void => {
  testStop?.()

  const target = [{ server: device.server, deviceIndex: device.index, actuatorIndex: actuator.index, actuatorType: actuator.actuatorType }]

  testing.value = motorKey(device, actuator.index)
  void api.toysSet(target, TEST_LEVEL)
  testStop = () => {
    clearTimeout(testTimer)
    void api.toysSet(target, 0)
    testing.value = null
    testStop = undefined
  }
  testTimer = setTimeout(() => testStop?.(), TEST_MS)
}

onBeforeUnmount(() => testStop?.())

const toggle = (
  device: IntifaceDevice,
  actuator: IntifaceDevice['actuators'][number],
  checked: boolean
): void => {
  if (checked) {
    if (isSelected(device, actuator.index)) return

    actuators.value = [
      ...actuators.value,
      {
        server: device.server,
        deviceIndex: device.index,
        deviceName: device.name,
        actuatorIndex: actuator.index,
        actuatorDescription: actuator.description,
        actuatorType: actuator.actuatorType
      }
    ]
  } else {
    actuators.value = actuators.value.filter((a) => !isMotor(a, device, actuator.index))
  }
}
</script>

<template>
  <UFormField
    label="Toys"
    :required="required ?? true"
    :ui="{ container: 'mt-3' }"
    :description="description ?? 'Pick the motors this control drives. A toy with several lists each one; the button beside it runs it for a second so you can tell which is which.'"
  >
    <div
      v-if="deviceList.length === 0"
      class="text-sm text-muted"
    >
      No toys yet. Make sure Intiface Central is running on one of your servers and a toy is paired.
    </div>
    <div
      v-else
      class="flex flex-wrap items-start gap-x-6 gap-y-4"
    >
      <div
        v-for="{ device, online } in deviceList"
        :key="toyKey(device.server, device.index)"
        class="grid gap-2"
      >
        <div class="flex items-center gap-2">
          <p class="text-sm font-medium text-highlighted">
            {{ device.name }}
            <span
              v-if="toyServerName(device.server)"
              class="font-normal text-muted"
            >· {{ toyServerName(device.server) }}</span>
          </p>
          <UBadge
            v-if="!online"
            color="neutral"
            variant="subtle"
            size="sm"
            label="Offline"
          />
        </div>
        <div class="flex flex-wrap gap-2">
          <CheckChip
            v-for="actuator in device.actuators"
            :id="`intiface-actuator-${motorKey(device, actuator.index)}`"
            :key="actuator.index"
            :label="actuatorLabel(device, actuator)"
            :model-value="isSelected(device, actuator.index)"
            @update:model-value="toggle(device, actuator, $event)"
          >
            <template #end>
              <UButton
                :disabled="!online"
                icon="mdi:vibrate"
                color="neutral"
                variant="ghost"
                size="lg"
                class="mr-1.5 rounded-full"
                :aria-label="`Test ${actuatorLabel(device, actuator)} on ${device.name}`"
                :title="online ? 'Test: runs it for a second' : 'Connect the toy to test it'"
                :loading="testing === motorKey(device, actuator.index)"
                @click="test(device, actuator)"
              />
            </template>
          </CheckChip>
        </div>
      </div>
    </div>
  </UFormField>
</template>

<style scoped></style>
