import type { ComponentPublicInstance, Ref } from 'vue'
import { onBeforeUnmount, onMounted, watch } from 'vue'

type MasonryTarget = HTMLElement | ComponentPublicInstance | null | undefined

// Masonry layout for a container of same-width cards (control groups), without changing how wide
// each card is: every card keeps its own definite, breakpoint-driven width (see ControlGroup.vue's
// w-108/2xl:w-164), this only decides where it sits.
//
// Not CSS multi-column (columns-*): its columns stretch to fill the container, so fixed-width cards
// inside them leave dead space, and its column-major order makes drag-to-reorder jump around.
// Not native `grid-template-rows: masonry` either - still behind flags. Instead, a CSS grid of
// max-content columns (so the cards themselves set the column width) with 1px auto rows, and each
// card explicitly placed into whichever column is currently shortest, spanning exactly its own
// height in rows - DOM order stays roughly row-major, which is what Sortable's reordering expects.
//
// The container should keep its flex-wrap classes as the pre-mount/SSR fallback; the inline grid
// styles set here override them once mounted.
export function useMasonry(target: Ref<MasonryTarget>, gap = 16): void {
  let container: HTMLElement | null = null
  let resizeObserver: ResizeObserver | null = null
  let widthObserver: ResizeObserver | null = null
  let mutationObserver: MutationObserver | null = null
  const observed = new Set<Element>()

  const items = (): HTMLElement[] =>
    Array.from(container?.children ?? []).filter(
      (el): el is HTMLElement =>
        el instanceof HTMLElement &&
        // Sortable's forceFallback drag clone gets appended into this same container, fixed-
        // positioned - it's not part of the layout and mustn't reserve a slot.
        el.style.position !== 'fixed' &&
        el.style.position !== 'absolute' &&
        getComputedStyle(el).display !== 'none'
    )

  const layout = (): void => {
    if (!container) return

    const children = items()
    if (!children.length) {
      container.style.gridTemplateColumns = ''
      return
    }

    const itemWidth = Math.max(...children.map((el) => el.offsetWidth))
    const available = container.clientWidth
    // Never more columns than cards - empty trailing columns would push justify-content: center
    // off-center, unlike the flex-wrap row this replaces.
    const columns = Math.max(
      1,
      Math.min(children.length, Math.floor((available + gap) / (itemWidth + gap)))
    )

    container.style.gridTemplateColumns = `repeat(${columns}, max-content)`

    // Next free 1px row line per column (grid lines are 1-based).
    const heights = Array<number>(columns).fill(1)

    for (const el of children) {
      let column = 0
      for (let i = 1; i < columns; i++) if (heights[i]! < heights[column]!) column = i

      const top = heights[column]!
      const span = Math.max(1, Math.ceil(el.getBoundingClientRect().height))
      el.style.gridColumn = `${column + 1}`
      el.style.gridRow = `${top} / span ${span}`
      // The gap is left as empty 1px rows before the next card rather than baked into this card's
      // span, so the last card in each column doesn't leave a trailing gap under the container.
      heights[column] = top + span + gap
    }
  }

  const syncObserved = (): void => {
    if (!container || !resizeObserver) return

    const current = new Set<Element>(container.children)
    for (const el of observed) {
      if (!current.has(el)) {
        resizeObserver.unobserve(el)
        observed.delete(el)
      }
    }
    for (const el of current) {
      if (!observed.has(el)) {
        resizeObserver.observe(el)
        observed.add(el)
      }
    }
  }

  // Sortable dispatches its `change` DOM event synchronously right after moving the dragged card,
  // and *before* it measures everyone's new position to animate the swap - re-laying out here (not
  // in the MutationObserver's later microtask) is what lets that animation see the final spots.
  const onSortableChange = (): void => layout()

  // What to watch for the available width changing. Not the container itself: its height is an
  // output of layout(), so observing it would make every card resize (e.g. a group collapsing)
  // re-trigger an observation at a shallower depth than the one being handled, which the browser
  // reports as a "ResizeObserver loop completed with undelivered notifications" error. The nearest
  // scroll container's size doesn't depend on our content (only its scrollbar appearing does, and
  // that changes its content-box width, which is exactly when a relayout is needed anyway).
  const scrollParent = (el: HTMLElement): HTMLElement => {
    for (let node = el.parentElement; node; node = node.parentElement) {
      if (/auto|scroll|overlay/.test(getComputedStyle(node).overflowY)) return node
    }
    return document.documentElement
  }

  const attach = (el: HTMLElement): void => {
    container = el
    Object.assign(el.style, {
      display: 'grid',
      gridAutoRows: '1px',
      rowGap: '0px',
      columnGap: `${gap}px`,
      justifyContent: 'center',
      // Cards must keep their natural height, not stretch to their row span - otherwise a
      // collapsing group would be measured at its old, stretched height and never shrink.
      alignItems: 'start'
    })

    resizeObserver = new ResizeObserver(() => layout())
    widthObserver = new ResizeObserver(() => layout())
    widthObserver.observe(scrollParent(el))
    mutationObserver = new MutationObserver(() => {
      syncObserved()
      layout()
    })
    mutationObserver.observe(el, { childList: true })
    el.addEventListener('change', onSortableChange)

    syncObserved()
    layout()
  }

  const detach = (): void => {
    resizeObserver?.disconnect()
    widthObserver?.disconnect()
    mutationObserver?.disconnect()
    container?.removeEventListener('change', onSortableChange)
    observed.clear()
    resizeObserver = null
    widthObserver = null
    mutationObserver = null
    container = null
  }

  const resolve = (value: MasonryTarget): HTMLElement | null => {
    const el = value && '$el' in value ? value.$el : value
    return el instanceof HTMLElement ? el : null
  }

  onMounted(() => {
    watch(
      () => resolve(target.value),
      (el) => {
        detach()
        if (el) attach(el)
      },
      { immediate: true, flush: 'post' }
    )
  })

  onBeforeUnmount(detach)
}
