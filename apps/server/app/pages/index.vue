<script setup lang="ts">
import { ControlBase, ControlBooleanBase, ControlOptions, ControlSliderBase } from '@toolkitosc/shared-ui'

useHead({
  title: 'ToolKitOSC',
  meta: [
    {
      name: 'description',
      content:
        "Bridge VRChat's OSC protocol with a desktop app, then share a link so anyone can control your avatar's parameters live from their browser."
    }
  ]
})

// Link-preview card for the homepage itself - the generic one drawn by server/routes/og-image.png.ts.
const origin = useRequestURL().origin

useSeoMeta({
  ogTitle: 'ToolKitOSC',
  ogDescription: 'Remote control panel for your VRChat avatar - share a link and let anyone drive your avatar parameters live.',
  ogImage: `${origin}/og-image.png`,
  ogImageWidth: 1200,
  ogImageHeight: 630,
  ogImageType: 'image/png',
  ogUrl: origin,
  ogType: 'website',
  twitterCard: 'summary_large_image'
})

const steps = [
  {
    icon: 'lucide:plug-zap',
    title: 'Connect the desktop app',
    description:
      "Install the Windows app and it hooks straight into VRChat's OSC input and output — nothing to set up beyond enabling OSC in VRChat."
  },
  {
    icon: 'lucide:layout-grid',
    title: 'Build your controls',
    description:
      'Map avatar parameters to toggles, sliders, enums, toy actuators or shockers. Group them, pick icons, and lock the ones nobody else should touch.'
  },
  {
    icon: 'lucide:share-2',
    title: 'Share a room link',
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

const { loggedIn } = useUserSession()

// Where Windows builds are published by the release workflow (.github/workflows/release-client.yml).
const DOWNLOAD_URL = 'https://github.com/NekoTiki/ToolKitOSC/releases/latest'

// The hero's demo: real control tiles on local state, so visitors can try the same controls their
// viewers will use.
const demoOn = ref(true)
const demoValue = ref(64)
const demoFace = ref(1)
const demoFaceOptions = computed(() => ['Happy', 'Blush', 'Smug', 'Sleepy'].map((name, index) => ({ key: name, name, selected: index === demoFace.value })))
</script>

<template>
  <div class="mx-auto grid w-full max-w-6xl gap-16 px-4 py-10 sm:px-7 lg:py-16">
    <section class="grid items-center gap-10 lg:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
      <div class="grid gap-5">
        <span class="font-mono text-xs tracking-widest text-secondary uppercase">Free &amp; open source</span>
        <h1 class="text-4xl leading-[1.05] font-semibold tracking-tight text-balance text-highlighted sm:text-5xl">
          Turn VRChat OSC into a <span class="text-primary">live control panel</span> you can share
        </h1>
        <p class="max-w-xl text-lg leading-relaxed text-muted">
          Build big, simple controls for your avatar in the desktop app, then send friends one link.
          Made to work inside VR, on a phone, or at a desk.
        </p>
        <div class="flex flex-wrap gap-2.5">
          <UButton
            v-if="loggedIn"
            size="xl"
            icon="i-lucide-link"
            to="/connect"
          >
            Connect the desktop app
          </UButton>
          <UButton
            v-else
            size="xl"
            icon="ic:baseline-discord"
            href="/auth/discord"
            target="_top"
            class="bg-[#5865f2] text-white hover:bg-[#4752c4]"
          >
            Sign in with Discord
          </UButton>
          <UButton
            size="xl"
            icon="i-lucide-download"
            color="neutral"
            variant="subtle"
            :to="DOWNLOAD_URL"
            target="_blank"
          >
            Download for Windows
          </UButton>
        </div>
      </div>

      <div class="grid justify-items-center gap-3 rounded-panel glass p-5">
        <span class="font-mono text-[10.5px] tracking-widest text-muted uppercase">Try it</span>
        <div
          class="tile-grid grid-cols-[repeat(2,var(--tile-cell))] [--tile-cell:10rem] max-sm:[--tile-cell:9rem]"
          data-density="m"
        >
          <ControlBooleanBase
            v-model="demoOn"
            title="Hoodie"
            icon="i-lucide-shirt"
          />
          <ControlSliderBase
            v-model="demoValue"
            title="Tail wag"
          />
          <div data-span="m">
            <ControlBase
              title="Face"
              kind="value"
              fill
            >
              <ControlOptions
                :options="demoFaceOptions"
                @select="demoFace = $event"
              />
            </ControlBase>
          </div>
        </div>
      </div>
    </section>

    <section class="grid gap-5">
      <div class="grid gap-1.5">
        <span class="font-mono text-xs tracking-widest text-secondary uppercase">How it works</span>
        <h2 class="text-3xl font-semibold tracking-tight text-highlighted">
          From OSC to a shareable panel in three steps
        </h2>
      </div>
      <div class="grid gap-3 md:grid-cols-3">
        <div
          v-for="(step, index) in steps"
          :key="step.title"
          class="grid content-start gap-2 rounded-panel glass p-5"
        >
          <span class="grid size-8.5 place-items-center rounded-full bg-primary/18 font-semibold text-primary">{{ index + 1 }}</span>
          <b class="text-base font-semibold text-highlighted">{{ step.title }}</b>
          <p class="text-[13.5px] leading-relaxed text-muted">
            {{ step.description }}
          </p>
        </div>
      </div>
    </section>

    <section class="grid gap-5">
      <div class="grid gap-1.5">
        <span class="font-mono text-xs tracking-widest text-secondary uppercase">Features</span>
        <h2 class="text-3xl font-semibold tracking-tight text-highlighted">
          Everything you need to run a room
        </h2>
      </div>
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <div
          v-for="feature in features"
          :key="feature.title"
          class="grid content-start gap-2 rounded-field glass p-4"
        >
          <UIcon
            :name="feature.icon"
            class="size-5.5 text-secondary"
          />
          <b class="text-[14.5px] font-semibold text-highlighted">{{ feature.title }}</b>
          <p class="text-[12.5px] leading-relaxed text-muted">
            {{ feature.description }}
          </p>
        </div>
      </div>
    </section>

    <section class="flex flex-wrap items-center gap-5 rounded-panel border border-default bg-linear-120 from-primary/25 to-secondary/18 p-7">
      <h2 class="min-w-56 flex-1 text-2xl font-semibold text-highlighted">
        Ready to share your first room?
      </h2>
      <UButton
        v-if="loggedIn"
        size="xl"
        to="/connect"
      >
        Connect the desktop app
      </UButton>
      <UButton
        v-else
        size="xl"
        icon="ic:baseline-discord"
        href="/auth/discord"
        target="_top"
        class="bg-[#5865f2] text-white hover:bg-[#4752c4]"
      >
        Sign in with Discord
      </UButton>
    </section>
  </div>
</template>
