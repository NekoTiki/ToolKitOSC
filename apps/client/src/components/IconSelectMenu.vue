<script setup lang="ts">
import type { SelectMenuItem } from '@nuxt/ui/components/SelectMenu.vue'
import { ref } from 'vue'

const getIconItem = (icon: string): { value: string; icon: string } => {
  return { value: icon, icon }
}

const icons = ref<SelectMenuItem[]>([
  [
    { type: 'label', label: 'Clothes' },
    { label: 'Top', ...getIconItem('game-icons:tank-top') },
    { label: 'T-Shirt', ...getIconItem('ion:shirt') },
    { label: 'Hoode', ...getIconItem('i-hugeicons:hoodie') },
    { label: 'Body', ...getIconItem('solar:body-bold') },
    { label: 'Corset', ...getIconItem('game-icons:corset') },
    { label: 'Skirt', ...getIconItem('solar:skirt-bold') },
    { label: 'Pants', ...getIconItem('icon-park-solid:baby-pants') },
    { label: 'Shorts', ...getIconItem('icon-park-solid:shorts') },
    { label: 'Underwear', ...getIconItem('hugeicons:underpants-01') },
    { label: 'Panties', ...getIconItem('icon-park-outline:panties') },
    { label: 'Bra', ...getIconItem('i-lucide-lab:bra-sports') },
    { label: 'Bikini', ...getIconItem('temaki:bikini') },
    { label: 'Bikini Color', ...getIconItem('emojione-v1:bikini') },
    { label: 'Sock', ...getIconItem('mingcute:sock-fill') },
    { label: 'Stocking', ...getIconItem('mdi:stocking') }
  ],
  [
    { type: 'label', label: 'Accessories' },
    { label: 'Shoe', ...getIconItem('mdi:shoe-formal') },
    { label: 'Heels', ...getIconItem('game-icons:high-heel') },
    { label: 'Boots', ...getIconItem('game-icons:steeltoe-boots') },
    { label: 'Running Shoes', ...getIconItem('hugeicons:running-shoes') },
    { label: 'Collar', ...getIconItem('game-icons:heavy-collar') },
    { label: 'Gloves', ...getIconItem('streamline-ultimate:chef-gear-gloves-bold') },
    { label: 'Watch', ...getIconItem('material-symbols:watch') },
    { label: 'Gun', ...getIconItem('fa7-solid:gun') },
    { label: 'Ribbon', ...getIconItem('streamline-sharp:medical-ribbon-1-solid') },
    { label: 'Ring', ...getIconItem('ri:diamond-ring-fill') },
    { label: 'Scarf', ...getIconItem('mingcute:scarf-fill') },
    { label: 'Mask', ...getIconItem('material-symbols:masks-rounded') },
    { label: 'Glasses', ...getIconItem('material-symbols:eyeglasses') },
    { label: 'Goggles', ...getIconItem('mdi:safety-googles') },
    { label: 'Horns', ...getIconItem('i-game-icons:bull-horns') }
  ],
  [
    { type: 'label', label: 'Body' },
    { label: 'Body', ...getIconItem('solar:body-shape-minimalistic-bold-duotone') },
    { label: 'Breast', ...getIconItem('healthicons:breasts') },
    { label: 'Breast', value: 'healthicons:breasts-outline', icon: 'healthicons:breasts-outline' },
    { label: 'Arm', ...getIconItem('game-icons:forearm') },
    { label: 'Legs', ...getIconItem('game-icons:female-legs') },
    { label: 'Feet', ...getIconItem('streamline-ultimate:medical-specialty-feet-bold') },
    { label: 'Animal Ears', ...getIconItem('emojione-monotone:cat-face') },
    { label: 'Ears', ...getIconItem('i-famicons:ear-outline') },
    { label: 'Tail', ...getIconItem('game-icons:fox-tail') }
  ],
  [
    { type: 'label', label: 'Others' },
    { label: 'Contrast', ...getIconItem('mingcute:shadow-fill') },
    { label: 'Contrast', ...getIconItem('material-symbols:contrast') },
    { label: 'Draw', ...getIconItem('material-symbols:draw-abstract') },
    { label: 'Makeup', ...getIconItem('icon-park-twotone:foundation-makeup') },
    { label: 'Dryer', ...getIconItem('ph:hair-dryer-fill') },
    { label: 'Eye Blind', ...getIconItem('streamline-flex:visual-blind-1') }
  ]
])

const model = defineModel<string>()
</script>

<template>
  <USelectMenu
    v-model="model"
    :leading-icon="model"
    :items="icons"
    value-key="value"
    virtualize
    placeholder="Select an icon"
  >
    <template
      v-if="model"
      #trailing
    >
      <UButton
        color="neutral"
        variant="link"
        size="sm"
        icon="i-lucide-circle-x"
        aria-label="Clear input"
        @click.prevent="model = ''"
      />
    </template>
  </USelectMenu>
</template>

<style scoped></style>
