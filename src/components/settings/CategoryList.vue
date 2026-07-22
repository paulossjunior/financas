<script setup lang="ts">
// Settings list of category groups; add/rename/delete groups and edit their keywords.
import { computed } from "vue";
import type { CategoryGroup } from "@/types/api.types";
import CategoryGroupEditor from "./CategoryGroupEditor.vue";

const props = defineProps<{ groups: CategoryGroup[] }>();
const emit = defineEmits<{
  "add-category": [];
  "delete-category": [name: string];
  "rename-category": [oldName: string, newName: string];
  "update-keywords": [categoryName: string, keywords: string[]];
}>();

const allKeywords = computed(() =>
  props.groups.flatMap((g) => g.keywords.map((k) => ({ keyword: k, category: g.name })))
);

function onRename(oldName: string, event: Event) {
  const newName = (event.target as HTMLInputElement).value.trim();
  if (newName && newName !== oldName) {
    emit("rename-category", oldName, newName);
  }
}

function onEditorUpdate(group: CategoryGroup) {
  emit("update-keywords", group.name, group.keywords);
}
</script>

<template>
  <div class="category-list">
    <div
      v-for="group in props.groups"
      :key="group.name"
      class="category-row"
      data-testid="category-row"
    >
      <div class="category-row-header">
        <input
          class="category-name-input"
          data-testid="category-name-input"
          :value="group.name"
          @change="onRename(group.name, $event)"
        />
        <button
          class="delete-btn"
          data-testid="delete-category-btn"
          @click="emit('delete-category', group.name)"
          title="Deletar categoria"
        >✕</button>
      </div>

      <CategoryGroupEditor
        :group="group"
        :allKeywords="allKeywords"
        @update="onEditorUpdate"
        @delete="emit('delete-category', $event)"
      />
    </div>

    <button class="add-btn" data-testid="add-category-btn" @click="emit('add-category')">
      + Nova Categoria
    </button>
  </div>
</template>

<style scoped>
.category-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.category-row {
  border-radius: var(--radius-lg, 6px);
  border: 1px solid var(--clr-stroke, #e0e0e0);
  background: var(--clr-surface, #f9f9f9);
  overflow: hidden;
}

.category-row-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--clr-stroke, #e0e0e0);
  background: var(--clr-bg, #fff);
}

.category-name-input {
  flex: 1;
  border: 1px solid transparent;
  background: transparent;
  font-size: 14px;
  font-weight: 600;
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--clr-text-primary, #1a1a1a);
}

.category-name-input:focus {
  outline: none;
  border-color: var(--clr-accent, #0078d4);
  background: var(--clr-bg, #fff);
}

.delete-btn {
  background: transparent;
  border: none;
  color: var(--clr-danger, #c42b1c);
  cursor: pointer;
  font-size: 14px;
  padding: 4px 8px;
  border-radius: 4px;
}

.delete-btn:hover { background: var(--clr-danger-light, #fde7e9); }

.add-btn {
  align-self: flex-start;
  margin-top: 4px;
  padding: 6px 14px;
  background: transparent;
  border: 1px dashed var(--clr-accent, #0078d4);
  color: var(--clr-accent, #0078d4);
  border-radius: var(--radius-lg, 6px);
  cursor: pointer;
  font-size: 13px;
}

.add-btn:hover { background: var(--clr-accent-light, #deecf9); }
</style>
