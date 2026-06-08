<script setup lang="ts">
const props = defineProps<{
  transactionId: string;
  currentCategory: string;
  availableCategories: string[];
  hasOverride: boolean;
}>();

const emit = defineEmits<{
  override: [transactionId: string, category: string];
  removeOverride: [transactionId: string];
}>();

function onSelect(event: Event) {
  const selected = (event.target as HTMLSelectElement).value;
  if (selected !== props.currentCategory) {
    emit("override", props.transactionId, selected);
  }
}
</script>

<template>
  <div class="tx-override">
    <span v-if="props.hasOverride" class="override-badge" data-testid="override-badge" title="Categoria sobreposta manualmente">✏</span>
    <select :value="props.currentCategory" class="cat-select" @change="onSelect">
      <option v-for="cat in props.availableCategories" :key="cat" :value="cat">{{ cat }}</option>
    </select>
    <button
      v-if="props.hasOverride"
      class="remove-override-btn"
      data-testid="remove-override-btn"
      @click="emit('removeOverride', props.transactionId)"
      title="Restaurar automático"
    >↺ Restaurar</button>
  </div>
</template>

<style scoped>
.tx-override {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.override-badge {
  font-size: 11px;
  color: var(--clr-accent, #0078d4);
}

.cat-select {
  font-size: 12px;
  border: 1px solid var(--clr-stroke, #d0d0d0);
  border-radius: 4px;
  padding: 2px 6px;
  background: var(--clr-bg, #fff);
  color: var(--clr-text-primary, #1a1a1a);
  cursor: pointer;
}

.remove-override-btn {
  font-size: 11px;
  background: transparent;
  border: none;
  color: var(--clr-text-muted, #666);
  cursor: pointer;
  padding: 0 4px;
  text-decoration: underline;
}

.remove-override-btn:hover { color: var(--clr-accent, #0078d4); }
</style>
