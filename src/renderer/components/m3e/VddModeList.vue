<template>
  <div class="vdd-modes" :data-kind="kind">
    <ol v-if="modelValue.length" class="vdd-modes__list">
      <li v-for="(value, index) in modelValue" :key="value" class="vdd-modes__row" :class="{ 'is-editing': editing === index }">
        <template v-if="editing === index">
          <input ref="editRef" v-model="editDraft" class="m-inline-input vdd-modes__input" :aria-label="texts.edit" :placeholder="texts.placeholder" inputmode="decimal" @keyup.enter="commitEdit" @keyup.esc="cancelEdit">
          <MIconButton icon="check" size="xs" :label="texts.save" @click="commitEdit" />
          <MIconButton icon="close" size="xs" :label="texts.cancel" @click="cancelEdit" />
        </template>
        <template v-else>
          <span class="vdd-modes__no">{{ index + 1 }}</span>
          <span class="vdd-modes__value">{{ shown(value) }}</span>
          <span class="vdd-modes__tools">
            <MIconButton icon="arrow_upward" size="xs" :label="texts.up" :disabled="index === 0" @click="move(index, -1)" />
            <MIconButton icon="arrow_downward" size="xs" :label="texts.down" :disabled="index === modelValue.length - 1" @click="move(index, 1)" />
            <MIconButton icon="edit" size="xs" :label="texts.edit" @click="startEdit(index)" />
            <MIconButton icon="delete" size="xs" :label="texts.remove" :disabled="modelValue.length <= 1" @click="remove(index)" />
          </span>
        </template>
      </li>
    </ol>
    <form class="vdd-modes__add" @submit.prevent="addDraft">
      <input v-model="draft" class="m-inline-input vdd-modes__input" :aria-label="texts.add" :placeholder="texts.placeholder" inputmode="decimal">
      <MButton size="sm" variant="tonal" icon="add" type="submit" :disabled="!draft.trim()">{{ texts.add }}</MButton>
      <MButton v-if="modelValue.length > 1" size="sm" variant="text" icon="sort" @click="sort">{{ texts.sort }}</MButton>
    </form>
    <div class="m-chips-edit m-chips-edit--suggest">
      <button v-for="s in suggestions" :key="s" type="button" class="m-chip m-chip--pick" :disabled="modelValue.includes(normalized(s))" @click="addValue(s)">{{ shown(s) }}</button>
    </div>
    <p v-if="error" class="vdd-modes__error" role="alert">{{ error }}</p>
  </div>
</template>

<script setup>
import { computed, nextTick, ref } from 'vue'
import { MButton, MIconButton } from 'foundation-ui/vue'
import { addEntry, editEntry, moveEntry, normalizers, removeEntry, sortList, RESOLUTION_LIMITS, RATE_LIMITS } from '../../composables/vddModeList.js'

// One ordered list of the driver's modes (resolutions or refresh rates): add, edit in place, remove, move, sort on request.
// The value is an array of strings ('1920x1080' | '59.94'); every change is emitted as a new array and the page decides when it is saved.
const props = defineProps({
  kind: { type: String, required: true, validator: (v) => v === 'resolution' || v === 'rate' },
  modelValue: { type: Array, required: true },
  suggestions: { type: Array, default: () => [] },
  /** add, placeholder, edit, save, cancel, up, down, remove, sort, errors: { invalid, range, exists, minOne, full } ('{min}' and '{max}' are filled in) */
  texts: { type: Object, required: true },
})
const emit = defineEmits(['update:modelValue'])

const draft = ref('')
const editing = ref(-1)
const editDraft = ref('')
const editRef = ref(null)
const errorCode = ref('')
const limits = computed(() => (props.kind === 'resolution' ? RESOLUTION_LIMITS : RATE_LIMITS))
const error = computed(() => (errorCode.value ? String(props.texts.errors[errorCode.value] || '').replace('{min}', limits.value.min).replace('{max}', limits.value.max) : ''))

const normalized = (v) => normalizers[props.kind](v).value ?? String(v)
const shown = (v) => (props.kind === 'resolution' ? String(v).replace('x', ' × ') : `${v} Hz`)
const apply = (result) => {
  errorCode.value = result.error || ''
  if (!result.error && result.list !== props.modelValue) emit('update:modelValue', result.list)
  return !result.error
}

const addValue = (text) => apply(addEntry(props.kind, props.modelValue, text))
const addDraft = () => { if (addValue(draft.value)) draft.value = '' }
const remove = (index) => apply(removeEntry(props.modelValue, index))
const move = (index, delta) => { errorCode.value = ''; emit('update:modelValue', moveEntry(props.modelValue, index, delta)) }
const sort = () => { errorCode.value = ''; emit('update:modelValue', sortList(props.kind, props.modelValue)) }
const startEdit = (index) => { errorCode.value = ''; editing.value = index; editDraft.value = props.modelValue[index]; nextTick(() => { const el = [].concat(editRef.value || [])[0]; el?.focus(); el?.select() }) }
const cancelEdit = () => { editing.value = -1; errorCode.value = '' }
const commitEdit = () => { if (apply(editEntry(props.kind, props.modelValue, editing.value, editDraft.value))) editing.value = -1 }
</script>

<style>
.vdd-modes__list { list-style: none; margin: 0 0 8px; padding: 0; display: flex; flex-direction: column; }
.vdd-modes__row { display: flex; align-items: center; gap: 8px; min-height: 40px; padding: 0 4px 0 12px; border-radius: 20px; }
.vdd-modes__row:hover { background: color-mix(in srgb, var(--md-sys-color-on-surface) 6%, transparent); }
.vdd-modes__no { width: 20px; color: var(--md-sys-color-on-surface-variant); font: var(--m-label-medium); font-variant-numeric: tabular-nums; }
.vdd-modes__value { flex: 1; font: var(--m-body-large); font-variant-numeric: tabular-nums; color: var(--md-sys-color-on-surface); }
.vdd-modes__tools { display: inline-flex; gap: 2px; }
.vdd-modes__add { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
.vdd-modes__input { width: 200px; }
.vdd-modes__error { margin: 8px 0 0; color: var(--md-sys-color-error); font: var(--m-body-small); }
</style>
