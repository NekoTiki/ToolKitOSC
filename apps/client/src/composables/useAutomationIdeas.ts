import { useAddons } from '@renderer/composables/useAddons'
import { useAvatarDetails } from '@renderer/composables/useAvatarDetails'
import { useBoards } from '@renderer/composables/useBoards'
import { useControls } from '@renderer/composables/useControls'
import { useLockedControls } from '@renderer/composables/useLockedControls'
import type { Automation } from '@renderer/lib/tauri-bridge'
import type { AutomationIdea, IdeaContext } from '@renderer/utils/automationIdeas'
import { AUTOMATION_IDEAS } from '@renderer/utils/automationIdeas'
import type { ComputedRef } from 'vue'
import { computed } from 'vue'

// The ideas you can start from right now, each built for the avatar you wear. Empty without one.
export function useAutomationIdeas(): { ideas: ComputedRef<(AutomationIdea & { automation: Automation })[]> } {
  const { avatarDetails } = useAvatarDetails()
  const { controls } = useControls()
  const { lockedControlGroups } = useLockedControls()
  const { boards } = useBoards()
  const { offAddonForAutomationKind, offAddonForControlType } = useAddons()

  const ideas = computed(() => {
    if (!avatarDetails.value) return []
    const context: IdeaContext = {
      avatarId: avatarDetails.value.id,
      avatarName: avatarDetails.value.name,
      controls: controls.value.flatMap((group) =>
        group.controls.filter((c) => !offAddonForControlType(c.type)).map((c) => ({ groupId: group.id, id: c.id, type: c.type }))
      ),
      profiles: lockedControlGroups.value,
      boards: boards.value
    }
    return AUTOMATION_IDEAS.filter((idea) => idea.kinds.every((kind) => !offAddonForAutomationKind(kind))).flatMap((idea) => {
      const automation = idea.build(context)
      return automation ? [{ ...idea, automation }] : []
    })
  })

  return { ideas }
}
