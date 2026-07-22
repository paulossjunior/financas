<script setup lang="ts">
// Settings editor for one category group — add/remove keywords with conflict detection.
import { ref, computed } from "vue";
import { detectConflict } from "@/utils/category-conflict";
import type { CategoryGroup } from "@/types/api.types";

const props = defineProps<{
  group: CategoryGroup;
  allKeywords: { keyword: string; category: string }[];
}>();

const emit = defineEmits<{
  update: [group: CategoryGroup];
  delete: [name: string];
}>();

const newKeyword = ref("");

const conflict = computed(() => {
  if (!newKeyword.value.trim()) return null;
  const groups: CategoryGroup[] = [];
  const seen = new Set<string>();
  for (const { category } of props.allKeywords) {
    if (!seen.has(category)) {
      seen.add(category);
      const kws = props.allKeywords.filter((k) => k.category === category).map((k) => k.keyword);
      groups.push({ name: category, keywords: kws, priority: 0 });
    }
  }
  return detectConflict(newKeyword.value, props.group.name, groups);
});

function addKeyword() {
  const kw = newKeyword.value.trim();
  if (!kw) return;
  const updated = { ...props.group, keywords: [...props.group.keywords, kw] };
  emit("update", updated);
  newKeyword.value = "";
}

function removeKeyword(kw: string) {
  const updated = { ...props.group, keywords: props.group.keywords.filter((k) => k !== kw) };
  emit("update", updated);
}
</script>

<template>
  <div class="group-editor">
    <div class="group-header">
      <span class="group-name">{{ props.group.name }}</span>
      <button
        class="delete-group-btn"
        data-testid="delete-group-btn"
        @click="emit('delete', props.group.name)"
        title="Deletar categoria"
      >✕</button>
    </div>

    <div class="keywords">
      <span
        v-for="kw in props.group.keywords"
        :key="kw"
        class="chip"
      >
        {{ kw }}
        <button
          class="remove-kw-btn"
          data-testid="remove-keyword-btn"
          @click="removeKeyword(kw)"
          :title="`Remover '${kw}'`"
        >×</button>
      </span>
    </div>

    <div class="add-kw-row">
      <input
        v-model="newKeyword"
        class="kw-input"
        data-testid="keyword-input"
        placeholder="Nova palavra-chave…"
        @input="() => {}"
        @keyup.enter="addKeyword"
      />
      <button
        class="add-kw-btn"
        data-testid="add-keyword-btn"
        @click="addKeyword"
      >+ Adicionar</button>
    </div>

    <div v-if="conflict" class="conflict-warning">
      ⚠ "{{ newKeyword }}" já usada em <strong>{{ conflict }}</strong>
    </div>
  </div>
</template>

<style scoped>
.group-editor {
  padding: 10px 12px;
  border: 1px solid var(--clr-border, #e0e0e0);
  border-radius: var(--radius-lg, 6px);
  background: var(--clr-surface, #f9f9f9);
}

.group-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.group-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--clr-text-primary, #1a1a1a);
}

.delete-group-btn {
  background: transparent;
  border: none;
  color: var(--clr-danger, #c42b1c);
  cursor: pointer;
  font-size: 14px;
  padding: 2px 6px;
  border-radius: 4px;
}

.delete-group-btn:hover { background: var(--clr-danger-light, #fde7e9); }

.keywords {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 8px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 12px;
  background: var(--clr-accent-light, #deecf9);
  color: var(--clr-accent, #0078d4);
  font-weight: 500;
}

.remove-kw-btn {
  background: transparent;
  border: none;
  color: var(--clr-accent, #0078d4);
  cursor: pointer;
  font-size: 13px;
  padding: 0;
  line-height: 1;
}

.add-kw-row {
  display: flex;
  gap: 6px;
}

.kw-input {
  flex: 1;
  padding: 4px 8px;
  font-size: 13px;
  border: 1px solid var(--clr-stroke, #d0d0d0);
  border-radius: 4px;
  outline: none;
}

.kw-input:focus { border-color: var(--clr-accent, #0078d4); }

.add-kw-btn {
  padding: 4px 10px;
  background: var(--clr-accent, #0078d4);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}

.conflict-warning {
  margin-top: 4px;
  font-size: 12px;
  color: var(--clr-warning, #e8a000);
}
</style>
