export default defineAppConfig({
  ui: {
    // Default theme until a host's own `theme-update` arrives (see useClientTheme.ts) - the same
    // pair as the desktop client's vite.config.ts, so nothing flickers when they match.
    colors: { primary: 'teal', secondary: 'orange', neutral: 'slate' },
    // Larger defaults: the share page is used from phones and from VR browsers with a laser
    // pointer, where the stock `md` controls are too small to hit reliably.
    button: { defaultVariants: { size: 'lg' } },
    // Aurora glass panels (see shared-ui's styles/aurora.css) instead of Nuxt UI's flat cards.
    card: { slots: { root: 'glass rounded-panel ring-0 divide-default' } },
    input: { defaultVariants: { size: 'lg' } },
    switch: { defaultVariants: { size: 'lg' } },
    // Every floating/portaled Nuxt UI component (Tooltip, Popover, DropdownMenu, Select,
    // SelectMenu, InputMenu) teleports its content to <body> with no z-index of its own, and
    // neither does Modal/Slideover - only UHeader ships a default (a sticky bar at z-50, see
    // app.vue). An explicit z-index always wins over z-index:auto content in the same stacking
    // context regardless of DOM/mount order, so left alone, the header would render on top of any
    // of these. Setting it once here, globally, fixes every instance - current and future -
    // instead of a per-usage `:ui="{ content: 'z-NN' }"` override each time a new one is added.
    //   50 - UHeader's sticky bar (framework default, not set here)
    //   60 - Modal / Slideover (need to beat the header)
    //   70 - Tooltip / Popover / DropdownMenu / Select content (need to beat both the header AND
    //        any Modal/Slideover they might be nested inside)
    modal: { slots: { overlay: 'z-[60]', content: 'z-[60]' } },
    slideover: { slots: { overlay: 'z-[60]', content: 'z-[60]' } },
    tooltip: { slots: { content: 'z-[70]' } },
    popover: { slots: { content: 'z-[70]' } },
    dropdownMenu: { slots: { content: 'z-[70]' } },
    select: { slots: { content: 'z-[70]' }, defaultVariants: { size: 'lg' } },
    selectMenu: { slots: { content: 'z-[70]' } },
    inputMenu: { slots: { content: 'z-[70]' } }
  }
})
