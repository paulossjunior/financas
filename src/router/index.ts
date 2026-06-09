import { createRouter, createWebHistory } from "vue-router";
import DashboardPage from "@/pages/DashboardPage.vue";
import HistoryPage from "@/pages/HistoryPage.vue";
import SettingsPage from "@/pages/SettingsPage.vue";
import TransactionsPage from "@/pages/TransactionsPage.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", component: DashboardPage },
    { path: "/transacoes", component: TransactionsPage },
    { path: "/historico", component: HistoryPage },
    { path: "/configuracoes", component: SettingsPage },
  ],
});
