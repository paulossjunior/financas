import { createRouter, createWebHistory } from "vue-router";
import DashboardPage from "@/pages/DashboardPage.vue";
import HistoryPage from "@/pages/HistoryPage.vue";
import SettingsPage from "@/pages/SettingsPage.vue";
import TransactionsPage from "@/pages/TransactionsPage.vue";
import ManualEntriesPage from "@/pages/ManualEntriesPage.vue";
import MappingPage from "@/pages/MappingPage.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", component: DashboardPage },
    { path: "/transacoes", component: TransactionsPage },
    { path: "/receitas-fixos", component: ManualEntriesPage },
    { path: "/mapeamento", component: MappingPage },
    { path: "/historico", component: HistoryPage },
    { path: "/configuracoes", component: SettingsPage },
  ],
});
