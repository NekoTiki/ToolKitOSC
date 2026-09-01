import type { ControlGroup } from '@vrc-osc-toolkit/shared-ui'

export const controlGroups: ControlGroup[] = [
  {
    id: 'main',
    name: 'Main Controls',
    controls: [
      {
        id: 'top',
        type: 'boolean',
        name: 'Top',
        icon: 'game-icons:tank-top',
        inputAddress: '/avatar/parameters/Top'
      },
      {
        id: 'bodysuit',
        type: 'boolean',
        name: 'Bodysuit',
        icon: 'solar:body-bold',
        inputAddress: '/avatar/parameters/Bodysuit'
      },
      {
        id: 'shirt',
        type: 'boolean',
        name: 'Shirt',
        icon: 'ion:shirt',
        inputAddress: '/avatar/parameters/Shirt'
      },
      {
        id: 'pasties',
        type: 'boolean',
        name: 'Pasties',
        icon: 'healthicons:breasts-outline',
        inputAddress: '/avatar/parameters/Pasties'
      },
      {
        id: 'bikini_top',
        type: 'boolean',
        name: 'Bikini Top',
        icon: 'temaki:bikini',
        inputAddress: '/avatar/parameters/Bikini_T'
      },
      {
        id: 'bikini_bottom',
        type: 'boolean',
        name: 'Bikini Bottom',
        icon: 'emojione-v1:bikini',
        inputAddress: '/avatar/parameters/Bikini_B'
      },
      {
        id: 'socks_stockings',
        type: 'boolean-enum',
        name: 'Socks & Stockings',
        icon: 'mingcute:sock-fill',
        inputs: [
          {
            id: 'barefoot',
            name: 'Barefoot',
            icon: 'streamline-ultimate:medical-specialty-feet-bold',
            inputAddress: {
              true: [],
              false: [
                '/avatar/parameters/Sock',
                '/avatar/parameters/Stocking',
                '/avatar/parameters/Garter1'
              ]
            }
          },
          {
            id: 'sock',
            name: 'Sock',
            icon: 'mingcute:sock-fill',
            inputAddress: {
              true: ['/avatar/parameters/Sock'],
              false: ['/avatar/parameters/Stocking', '/avatar/parameters/Garter1']
            }
          },
          {
            id: 'stocking',
            name: 'Stocking',
            icon: 'mdi:stocking',
            inputAddress: {
              true: ['/avatar/parameters/Stocking', '/avatar/parameters/Garter1'],
              false: ['/avatar/parameters/Sock']
            }
          },
          {
            id: 'both',
            name: 'Both',
            icon: 'mdi:stocking',
            inputAddress: {
              true: [
                '/avatar/parameters/Sock',
                '/avatar/parameters/Stocking',
                '/avatar/parameters/Garter1'
              ],
              false: []
            }
          }
        ]
      },
      {
        id: 'bottom',
        type: 'enum',
        name: 'Bottom',
        icon: 'icon-park-solid:baby-pants',
        inputAddress: '/avatar/parameters/Bottoms',
        options: [
          { name: 'None', icon: 'healthicons:breasts', value: 0 },
          { name: 'Shorts', icon: 'icon-park-solid:shorts', value: 1 },
          { name: 'Skirt', icon: 'emojione-v1:bikini', value: 2 },
          { name: 'Pants', icon: 'icon-park-solid:baby-pants', value: 3 }
        ]
      },
      {
        id: 'fishnet',
        type: 'boolean',
        name: 'Fishnet',
        icon: 'icon-park-solid:baby-pants',
        inputAddress: '/avatar/parameters/Fishnet'
      },
      {
        id: 'shoes',
        type: 'boolean-enum',
        name: 'Shoes',
        icon: 'mdi:shoe-formal',
        inputs: [
          {
            id: 'barefoot',
            name: 'Barefoot',
            icon: 'streamline-ultimate:medical-specialty-feet-bold',
            inputAddress: {
              true: [],
              false: ['/avatar/parameters/Boots', '/avatar/parameters/Sneaker']
            }
          },
          {
            id: 'boots',
            name: 'Boots',
            icon: 'game-icons:high-heel',
            inputAddress: {
              true: ['/avatar/parameters/Boots'],
              false: ['/avatar/parameters/Sneaker']
            }
          },
          {
            id: 'sneaker',
            name: 'Sneaker',
            icon: 'hugeicons:running-shoes',
            inputAddress: {
              true: ['/avatar/parameters/Sneaker'],
              false: ['/avatar/parameters/Boots']
            }
          }
        ]
      }
    ]
  },
  {
    id: 'accessories',
    name: 'Accessories',
    controls: [
      {
        id: 'choker_collar',
        type: 'boolean-enum',
        name: 'Choker & Collar',
        icon: 'game-icons:heavy-collar',
        inputs: [
          {
            id: 'none',
            name: 'None',
            icon: '',
            inputAddress: {
              true: [],
              false: ['/avatar/parameters/Choker', '/avatar/parameters/Collar']
            }
          },
          {
            id: 'choker',
            name: 'Choker',
            icon: 'game-icons:heavy-collar',
            inputAddress: {
              true: ['/avatar/parameters/Choker'],
              false: ['/avatar/parameters/Collar']
            }
          },
          {
            id: 'collar',
            name: 'Collar',
            icon: 'game-icons:heavy-collar',
            inputAddress: {
              true: ['/avatar/parameters/Collar'],
              false: ['/avatar/parameters/Choker']
            }
          },
          {
            id: 'both',
            name: 'Both',
            icon: 'game-icons:heavy-collar',
            inputAddress: {
              true: ['/avatar/parameters/Choker', '/avatar/parameters/Collar'],
              false: []
            }
          }
        ]
      },
      {
        id: 'gloves',
        type: 'boolean',
        name: 'Gloves',
        icon: 'streamline-ultimate:chef-gear-gloves-bold',
        inputAddress: '/avatar/parameters/Gloves'
      },
      {
        id: 'gun',
        type: 'boolean',
        name: 'Gun',
        icon: 'fa7-solid:gun',
        reverse: true,
        inputAddress: '/avatar/parameters/VF114_Gun_Toggle'
      },
      {
        id: 'goggles',
        type: 'boolean',
        name: 'Goggles',
        icon: 'mdi:safety-googles',
        inputAddress: '/avatar/parameters/Goggles'
      },
      {
        id: 'scarf',
        type: 'boolean',
        name: 'Scarf',
        icon: 'mingcute:scarf-fill',
        inputAddress: '/avatar/parameters/Scarf'
      },
      {
        id: 'mask',
        type: 'boolean',
        name: 'Mask',
        icon: 'material-symbols:masks-rounded',
        reverse: true,
        inputAddress: '/avatar/parameters/Mask'
      },
      {
        id: 'arm_warmer',
        type: 'boolean',
        name: 'Arm Warmer',
        icon: 'game-icons:forearm',
        inputAddress: '/avatar/parameters/ArmWarm'
      },
      {
        id: 'straps',
        type: 'boolean-group',
        name: 'Straps',
        icon: 'ph:eyeglasses-bold',
        inputs: [
          { name: 'Arm Wrap', inputAddress: '/avatar/parameters/ArmWrap' },
          { name: 'Waist', inputAddress: '/avatar/parameters/WaistStrap' },
          { name: 'Garter', inputAddress: '/avatar/parameters/Garter2' },
          { name: 'Leg', inputAddress: '/avatar/parameters/LegStrap' }
        ]
      }
    ]
  },
  {
    id: 'appearance',
    name: 'Appearance',
    controls: [
      {
        id: 'booba',
        type: 'slider',
        name: 'Booba',
        icon: 'healthicons:breasts',
        inputAddress: '/avatar/parameters/booba'
      },
      {
        id: 'skin_saturation',
        type: 'slider',
        name: 'Skin Sat',
        icon: 'solar:body-shape-minimalistic-bold-duotone',
        inputAddress: '/avatar/parameters/SkinSat'
      },
      {
        id: 'skin_tone',
        type: 'slider',
        name: 'Skin Tone',
        icon: 'solar:body-shape-minimalistic-bold-duotone',
        inputAddress: '/avatar/parameters/Skintone'
      },
      {
        id: 'face_shadow',
        type: 'slider',
        name: 'Face Shadow',
        icon: 'mingcute:shadow-fill',
        locked: true,
        inputAddress: '/avatar/parameters/Face_Shadow'
      },
      {
        id: 'tatoos',
        type: 'boolean',
        name: 'Tatoos',
        icon: 'material-symbols:draw-abstract',
        inputAddress: '/avatar/parameters/Tats'
      },
      {
        id: 'black_or_white',
        type: 'boolean',
        name: 'Black or White',
        icon: 'material-symbols:contrast',
        inputAddress: '/avatar/parameters/Clothes_B&W'
      },
      {
        id: 'black_or_white_nails',
        type: 'boolean',
        name: 'Nails Black or White',
        icon: 'material-symbols:contrast',
        reverse: true,
        inputAddress: '/avatar/parameters/Nails'
      },
      {
        id: 'face_greasepaint',
        type: 'boolean',
        name: 'Face Greasepaint',
        icon: 'icon-park-twotone:foundation-makeup',
        inputAddress: '/avatar/parameters/GreasePaint'
      },
      {
        id: 'hairstyle',
        type: 'enum',
        name: 'Hair',
        icon: 'ph:hair-dryer-fill',
        inputAddress: '/avatar/parameters/Hair_Back',
        options: [
          { name: 'Long', value: 0 },
          { name: 'Pony', value: 1 },
          { name: 'Short', value: 2 },
          { name: 'Pigtail', value: 3 }
        ]
      },
      {
        id: 'bangs',
        type: 'enum',
        name: 'Bangs',
        icon: 'ph:hair-dryer-fill',
        inputAddress: '/avatar/parameters/Hair_Front',
        options: [
          { name: 'Basic', value: 3 },
          { name: 'Swept', value: 0 },
          { name: 'Fluffy', value: 1 },
          { name: 'Short', value: 2 }
        ]
      },
      {
        id: 'ears',
        type: 'boolean',
        name: 'Ears',
        icon: 'emojione-monotone:cat-face',
        inputAddress: '/avatar/parameters/Ears'
      },
      {
        id: 'tail',
        type: 'boolean',
        name: 'Tail',
        icon: 'game-icons:fox-tail',
        inputAddress: '/avatar/parameters/Tail'
      }
    ]
  },
  {
    id: 'other',
    name: 'Other',
    controls: [
      {
        id: 'open-shock-shocker',
        type: 'open-shock-shocker',
        name: 'Shock',
        mode: 'Shock',
        shockers: ['0197d6b0-cabe-7679-abbf-3a9756456cd3', '0197d6b2-9821-7ee4-97ad-d4bc5beb0413'],
        intensity: {
          min: 0,
          max: 100
        },
        duration: {
          min: 300,
          max: 5000
        },
        cooldown: 10000,
        animationDuration: 3000
      }
    ]
  }
]
