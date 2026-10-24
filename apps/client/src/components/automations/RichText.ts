import type { Rich } from '@renderer/utils/automationSentence'
import type { FunctionalComponent } from 'vue'
import { h } from 'vue'

// A piece of an automation's sentence: plain text with its names and values in <b>, which the
// parent styles (see AutomationSentence.vue, EasySentence.vue).
const RichText: FunctionalComponent<{ parts: Rich }> = (props) =>
  props.parts.map((part) => (part.strong ? h('b', part.text) : part.text))
RichText.props = ['parts']

export default RichText
