<script setup lang="ts">
import type { TableColumn } from '@nuxt/ui/components/Table.vue'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useOscMessages } from '@renderer/composables/useOscMessages'
import type { OSCArg } from '@renderer/env'
import { type Column, getPaginationRowModel } from '@tanstack/vue-table'
import { computed, h, ref, useTemplateRef } from 'vue'

type OscData = {
  name: string
  inputAddress?: string
  inputType?: string
  outputType?: string
}

const { avatarDetails } = useAvatarDetails()
const { args, update } = useOscMessages()

const table = useTemplateRef('table')

const sorting = ref([{ id: 'name', desc: false }])

const getSortableHeader =
  (label: string) =>
  ({ column }: { column: Column<OscData> }): object => {
    const isSorted = column.getIsSorted()

    return h(UButton, {
      color: 'neutral',
      variant: 'ghost',
      label,
      icon: isSorted
        ? isSorted === 'asc'
          ? 'i-lucide-arrow-up-narrow-wide'
          : 'i-lucide-arrow-down-wide-narrow'
        : 'i-lucide-arrow-up-down',
      class: '-mx-2.5',
      onClick: () => column.toggleSorting(column.getIsSorted() === 'asc')
    })
  }

const oscColumns: TableColumn<OscData>[] = [
  { accessorKey: 'name', header: getSortableHeader('Name') },
  { accessorKey: 'actions', header: () => h('div', { class: 'text-right' }, 'Actions') }
]

const showAll = ref(false)
const globalFilter = ref('')

const getCurrentValue = (data: OscData): OSCArg[] => {
  if (!data.inputAddress) return getDefaultArgs(data.inputType)

  return args.value?.[data.inputAddress] || getDefaultArgs(data.inputType)
}

const getDefaultArgs = (type: string | undefined): OSCArg[] => {
  switch (type) {
    case 'Bool':
      return [false]
    case 'Int':
      return [0]
    case 'Float':
      return [0.0]
    default:
      return []
  }
}

const oscRows = computed<OscData[]>(() => {
  const filteredList =
    avatarDetails.value?.parameters?.filter((parameter) => {
      if (showAll.value) return true

      const name = parameter.name

      if (!name) return false

      if (name.startsWith('pcs/')) return false
      else if (name.startsWith('OGB/')) return false
      else if (name.startsWith('Go/')) return false
      else if (name.startsWith('WH Lollipop/')) return false
      else if (name.startsWith('FT/')) return false
      else if (name.startsWith('Dor/')) return false
      else if (name.match(/VF\d+_/)) return false

      return true
    }) || []

  return filteredList.map((param) => ({
    name: param.name,
    inputAddress: param.input?.address,
    inputType: param.input?.type,
    outputType: param.output?.type
  }))
})
</script>

<template>
  <UTable
    ref="table"
    v-model:global-filter="globalFilter"
    v-model:sorting="sorting"
    class="max-h-full"
    :data="oscRows"
    :columns="oscColumns"
    sticky
    :pagination-options="{ getPaginationRowModel: getPaginationRowModel() }"
  >
    <template #actions-cell="{ row }">
      <div class="flex justify-end gap-2">
        <UButton
          icon="i-lucide-refresh-cw"
          color="primary"
          size="xs"
          @click="
            update({
              address: row.original.inputAddress as string,
              args: []
            })
          "
        >
          Fetch
        </UButton>

        <USwitch
          v-if="row.original.inputType === 'Bool'"
          :model-value="getCurrentValue(row.original)[0] as boolean"
          @update:model-value="
            (val) =>
              update({
                address: row.original.inputAddress as string,
                args: [val]
              })
          "
        />
        <USlider
          v-if="row.original.inputType === 'Float'"
          :model-value="getCurrentValue(row.original)[0] as number"
          :min="0"
          :max="1"
          :step="0.01"
          tooltip
          @update:model-value="
            (val) => {
              if (typeof val !== 'number') return

              update({
                address: row.original.inputAddress as string,
                args: [val as number]
              })
            }
          "
        />
        <div
          v-if="row.original.inputType === 'Int'"
          class="flex items-center"
        >
          <UButton
            v-if="row.original.inputType === 'Int'"
            :disabled="(getCurrentValue(row.original)[0] as number) <= 0"
            icon="i-lucide-minus"
            color="primary"
            size="xs"
            @click="
              update({
                address: row.original.inputAddress as string,
                args: [Math.max(0, (getCurrentValue(row.original)?.[0] as number) - 1)]
              })
            "
          />
          <span class="px-2">{{ getCurrentValue(row.original)?.[0] }}</span>
          <UButton
            v-if="row.original.inputType === 'Int'"
            icon="i-lucide-plus"
            color="primary"
            size="xs"
            @click="
              update({
                address: row.original.inputAddress as string,
                args: [(getCurrentValue(row.original)?.[0] as number) + 1]
              })
            "
          />
        </div>
        <div
          v-if="!row.original.inputType"
          class="text-neutral-500"
        >
          {{ getCurrentValue(row.original)?.[0] }}
        </div>
      </div>
    </template>
  </UTable>
</template>

<style scoped></style>
