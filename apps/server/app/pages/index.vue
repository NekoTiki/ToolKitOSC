<script setup lang="ts">
useHead({
  title: 'VRC OSC Toolkit',
  meta: [
    {
      name: 'description',
      content:
        "Bridge VRChat's OSC protocol with a desktop app, then share a link so anyone can control your avatar's parameters live from their browser."
    }
  ]
})

const steps = [
  {
    icon: 'lucide:plug-zap',
    title: '1. Connect the desktop app',
    description:
      "Install the Windows app and it hooks straight into VRChat's OSC input and output — nothing to set up beyond enabling OSC in VRChat."
  },
  {
    icon: 'lucide:layout-grid',
    title: '2. Build your controls',
    description:
      'Map avatar parameters to toggles, sliders, enums, toy actuators or shockers. Group them, pick icons, and lock the ones nobody else should touch.'
  },
  {
    icon: 'lucide:share-2',
    title: '3. Share a room link',
    description:
      'Every room gets its own URL. Open it in any browser for a live control panel kept in sync over a WebSocket connection.'
  }
]

const features = [
  {
    icon: 'lucide:cable',
    title: 'Real-time OSC bridge',
    description:
      "A two-way bridge to VRChat's OSC protocol — parameter changes reach viewers instantly, and their commands are sent straight back to your avatar."
  },
  {
    icon: 'lucide:sliders-horizontal',
    title: 'A control for every parameter',
    description:
      'Toggles, toggle groups, multi-state toggle logic, enums, step-enums and sliders, each with its own icon and mapped to any OSC address.'
  },
  {
    icon: 'lucide:share-2',
    title: 'Shareable remote rooms',
    description:
      'Generate a link and hand it out — viewers get a live dashboard in their browser, nothing to install, nothing to configure.'
  },
  {
    icon: 'ic:baseline-discord',
    title: 'Discord sign-in & access control',
    description:
      'Gate a room behind Discord, capture guest display names, lock specific control groups, and ban a disruptive viewer by Discord account or IP.'
  },
  {
    icon: 'lucide:waves',
    title: 'Intiface / Buttplug toys',
    description:
      "Drive any connected toy's actuators from a control, or offer a chip-picker of curated, timed vibration patterns viewers can trigger like a preset."
  },
  {
    icon: 'lucide:zap',
    title: 'OpenShock shockers',
    description:
      'Wire a shock or vibrate control to your OpenShock shockers, with configurable intensity, duration and cooldown so limits stay in your hands.'
  },
  {
    icon: 'lucide:activity',
    title: 'Live status & history',
    description:
      "See who's connected, watch a running command log, and check a last-used-by avatar on every control."
  },
  {
    icon: 'lucide:monitor',
    title: 'Lightweight desktop client',
    description: 'A native Windows app that stays out of the way — OSC in, controls out.'
  }
]
</script>

<template>
  <div>
    <UPageHero
      headline="Free & open source"
      title="Turn VRChat OSC into a live control panel you can share"
      description="VRC OSC Toolkit pairs a desktop bridge for VRChat's OSC protocol with this website: build controls for your avatar's parameters, then hand out a link so anyone can drive them live from their browser."
      :links="[
        {
          label: 'Sign in with Discord',
          icon: 'ic:baseline-discord',
          href: '/auth/discord',
          target: '_top',
          size: 'xl'
        }
      ]"
    />

    <UPageSection
      headline="How it works"
      title="From OSC to a shareable panel in three steps"
      :features="steps"
    />

    <UPageSection
      headline="Features"
      title="Everything you need to run a room"
      description="From avatar toggles to toys and shockers, the toolkit covers the whole path between VRChat and the people you share it with."
      :features="features"
      :ui="{ features: 'sm:grid-cols-2 lg:grid-cols-4' }"
    />

    <UPageCTA
      title="Ready to share your first room?"
      description="Sign in with Discord — it only takes a few seconds, and you can build your first control right after."
      :links="[
        {
          label: 'Sign in with Discord',
          icon: 'ic:baseline-discord',
          href: '/auth/discord',
          target: '_top',
          size: 'lg'
        }
      ]"
      variant="subtle"
    />
  </div>
</template>
