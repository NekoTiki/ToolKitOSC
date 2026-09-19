// Single source of truth for the curated icon set a control can be given - both the client's icon
// picker (IconSelectMenu.vue, grouped by `category`) and the AI control-suggestion feature (server
// prompt.ts, which needs a closed vocabulary it can safely constrain a model's output to) read
// from this instead of keeping their own separate lists, so the two can never drift out of sync.
export interface ControlIconOption {
  category: 'Clothes' | 'Accessories' | 'Body' | 'Others'
  label: string
  icon: string
}

export const CONTROL_ICONS: ControlIconOption[] = [
  // Clothes
  { category: 'Clothes', label: 'Top', icon: 'game-icons:tank-top' },
  { category: 'Clothes', label: 'T-Shirt', icon: 'ion:shirt' },
  { category: 'Clothes', label: 'Hoodie', icon: 'i-hugeicons:hoodie' },
  { category: 'Clothes', label: 'Body', icon: 'solar:body-bold' },
  { category: 'Clothes', label: 'Corset', icon: 'game-icons:corset' },
  { category: 'Clothes', label: 'Skirt', icon: 'solar:skirt-bold' },
  { category: 'Clothes', label: 'Pants', icon: 'icon-park-solid:baby-pants' },
  { category: 'Clothes', label: 'Shorts', icon: 'icon-park-solid:shorts' },
  { category: 'Clothes', label: 'Underwear', icon: 'hugeicons:underpants-01' },
  { category: 'Clothes', label: 'Panties', icon: 'icon-park-outline:panties' },
  { category: 'Clothes', label: 'Bra', icon: 'i-lucide-lab:bra-sports' },
  { category: 'Clothes', label: 'Bikini', icon: 'temaki:bikini' },
  { category: 'Clothes', label: 'Bikini Color', icon: 'emojione-v1:bikini' },
  { category: 'Clothes', label: 'Sock', icon: 'mingcute:sock-fill' },
  { category: 'Clothes', label: 'Stocking', icon: 'mdi:stocking' },
  { category: 'Clothes', label: 'Dress', icon: 'mingcute:dress-fill' },
  { category: 'Clothes', label: 'Kimono', icon: 'game-icons:kimono' },
  { category: 'Clothes', label: 'Sweater', icon: 'icon-park-solid:sweater' },
  { category: 'Clothes', label: 'Coat', icon: 'icon-park-outline:women-coat' },
  { category: 'Clothes', label: 'Vest', icon: 'icon-park-solid:vest' },
  { category: 'Clothes', label: 'Swimsuit', icon: 'icon-park-solid:swimsuit' },
  { category: 'Clothes', label: 'Onesie', icon: 'icon-park-solid:onesies' },
  { category: 'Clothes', label: 'Cape', icon: 'game-icons:cape' },
  { category: 'Clothes', label: 'Apron', icon: 'hugeicons:apron' },

  // Accessories
  { category: 'Accessories', label: 'Shoe', icon: 'mdi:shoe-formal' },
  { category: 'Accessories', label: 'Heels', icon: 'game-icons:high-heel' },
  { category: 'Accessories', label: 'Boots', icon: 'game-icons:steeltoe-boots' },
  { category: 'Accessories', label: 'Running Shoes', icon: 'hugeicons:running-shoes' },
  { category: 'Accessories', label: 'Collar', icon: 'game-icons:heavy-collar' },
  { category: 'Accessories', label: 'Gloves', icon: 'streamline-ultimate:chef-gear-gloves-bold' },
  { category: 'Accessories', label: 'Watch', icon: 'material-symbols:watch' },
  { category: 'Accessories', label: 'Gun', icon: 'fa7-solid:gun' },
  { category: 'Accessories', label: 'Ribbon', icon: 'streamline-sharp:medical-ribbon-1-solid' },
  { category: 'Accessories', label: 'Ring', icon: 'ri:diamond-ring-fill' },
  { category: 'Accessories', label: 'Scarf', icon: 'mingcute:scarf-fill' },
  { category: 'Accessories', label: 'Mask', icon: 'material-symbols:masks-rounded' },
  { category: 'Accessories', label: 'Glasses', icon: 'material-symbols:eyeglasses' },
  { category: 'Accessories', label: 'Goggles', icon: 'mdi:safety-googles' },
  { category: 'Accessories', label: 'Horns', icon: 'i-game-icons:bull-horns' },
  { category: 'Accessories', label: 'Hat', icon: 'icon-park-solid:hat' },
  { category: 'Accessories', label: 'Cap', icon: 'hugeicons:cap' },
  { category: 'Accessories', label: 'Crown', icon: 'solar:crown-bold' },
  { category: 'Accessories', label: 'Necklace', icon: 'game-icons:necklace' },
  { category: 'Accessories', label: 'Earrings', icon: 'game-icons:earrings' },
  { category: 'Accessories', label: 'Bandana', icon: 'game-icons:bandana' },
  { category: 'Accessories', label: 'Belt', icon: 'game-icons:belt' },
  { category: 'Accessories', label: 'Bow Tie', icon: 'mdi:bow-tie' },
  { category: 'Accessories', label: 'Backpack', icon: 'solar:backpack-bold' },
  { category: 'Accessories', label: 'Headphones', icon: 'material-symbols:headphones' },
  { category: 'Accessories', label: 'Hair', icon: 'mingcute:hair-fill' },
  { category: 'Accessories', label: 'Eyepatch', icon: 'game-icons:eyepatch' },
  { category: 'Accessories', label: 'Chain', icon: 'fa7-solid:chain' },
  { category: 'Accessories', label: 'Feather', icon: 'ph:feather-fill' },
  { category: 'Accessories', label: 'Halo', icon: 'icon-park-solid:halo' },

  // Body
  { category: 'Body', label: 'Body', icon: 'solar:body-shape-minimalistic-bold-duotone' },
  { category: 'Body', label: 'Breast', icon: 'healthicons:breasts' },
  { category: 'Body', label: 'Breast (Outline)', icon: 'healthicons:breasts-outline' },
  { category: 'Body', label: 'Arm', icon: 'game-icons:forearm' },
  { category: 'Body', label: 'Legs', icon: 'game-icons:female-legs' },
  { category: 'Body', label: 'Feet', icon: 'streamline-ultimate:medical-specialty-feet-bold' },
  { category: 'Body', label: 'Animal Ears', icon: 'emojione-monotone:cat-face' },
  { category: 'Body', label: 'Ears', icon: 'i-famicons:ear-outline' },
  { category: 'Body', label: 'Tail', icon: 'game-icons:fox-tail' },
  { category: 'Body', label: 'Angel Wings', icon: 'game-icons:feathered-wing' },
  { category: 'Body', label: 'Bat Wings', icon: 'game-icons:bat-wing' },
  { category: 'Body', label: 'Paw', icon: 'mdi:paw' },
  { category: 'Body', label: 'Claw', icon: 'game-icons:claw' },
  { category: 'Body', label: 'Fangs', icon: 'game-icons:fangs' },
  { category: 'Body', label: 'Hoof', icon: 'game-icons:hoof' },
  { category: 'Body', label: 'Fin', icon: 'mdi:shark-fin' },
  { category: 'Body', label: 'Mustache', icon: 'game-icons:mustache' },
  { category: 'Body', label: 'Beard', icon: 'game-icons:beard' },

  // Others
  { category: 'Others', label: 'Contrast', icon: 'mingcute:shadow-fill' },
  { category: 'Others', label: 'Contrast (Alt)', icon: 'material-symbols:contrast' },
  { category: 'Others', label: 'Draw', icon: 'material-symbols:draw-abstract' },
  { category: 'Others', label: 'Makeup', icon: 'icon-park-twotone:foundation-makeup' },
  { category: 'Others', label: 'Dryer', icon: 'ph:hair-dryer-fill' },
  { category: 'Others', label: 'Eye Blind', icon: 'streamline-flex:visual-blind-1' },
  { category: 'Others', label: 'Tattoo', icon: 'temaki:tattoo-machine' },
  { category: 'Others', label: 'Lipstick', icon: 'icon-park-solid:lipstick' },
  { category: 'Others', label: 'Sparkle', icon: 'ph:sparkle-bold' },
  { category: 'Others', label: 'Flame', icon: 'solar:flame-bold' },
  { category: 'Others', label: 'Lightning', icon: 'solar:lightning-bold' },
  { category: 'Others', label: 'LED', icon: 'icon-park-solid:led-diode' },

  // New additions - functional categories the original picker had gaps for (surfaced by real
  // avatar parameter names seen while building the AI feature: eye/sclera sliders, lighting
  // brightness, SFX toggles, color pickers, step-enum "reset" controls - none of the above icons
  // fit those well; picked from the same collections already used elsewhere in this list for
  // visual consistency).
  { category: 'Others', label: 'Eye', icon: 'ph:eye-fill' },
  { category: 'Others', label: 'Eye Color', icon: 'material-symbols:visibility' },
  { category: 'Others', label: 'Light Brightness', icon: 'ph:sun-fill' },
  { category: 'Others', label: 'Light Bulb', icon: 'ph:lightbulb-filament-fill' },
  { category: 'Others', label: 'Color Palette', icon: 'ph:palette-fill' },
  { category: 'Others', label: 'Volume', icon: 'material-symbols:volume-up-rounded' },
  { category: 'Others', label: 'Music Note', icon: 'ph:music-notes-fill' },
  { category: 'Others', label: 'Reset', icon: 'material-symbols:restart-alt' },
  { category: 'Others', label: 'Toggle Switch', icon: 'material-symbols:toggle-on' },
  { category: 'Others', label: 'Scale/Size', icon: 'ph:ruler-fill' },
  { category: 'Others', label: 'Droplet', icon: 'ph:drop-fill' },
  { category: 'Others', label: 'Wind/Physics', icon: 'ph:wind-fill' }
]
