import { createRouter, createWebHistory } from "vue-router";
import DashboardPage from "@/pages/DashboardPage.vue";
import YearPage from "@/pages/YearPage.vue";
import HistoryPage from "@/pages/HistoryPage.vue";
import SettingsPage from "@/pages/SettingsPage.vue";
import TransactionsPage from "@/pages/TransactionsPage.vue";
import ManualEntriesPage from "@/pages/ManualEntriesPage.vue";
import MappingPage from "@/pages/MappingPage.vue";
import ContrachequePage from "@/pages/ContrachequePage.vue";
import ExtratoPage from "@/pages/ExtratoPage.vue";
import ImportsPage from "@/pages/ImportsPage.vue";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", component: DashboardPage },
    { path: "/ano", component: YearPage },
    { path: "/transacoes", component: TransactionsPage },
    { path: "/receitas-fixos", component: ManualEntriesPage },
    { path: "/contracheque", component: ContrachequePage },
    { path: "/importacoes", component: ImportsPage },
    { path: "/extrato", component: ExtratoPage },
    { path: "/mapeamento", component: MappingPage },
    { path: "/historico", component: HistoryPage },
    { path: "/configuracoes", component: SettingsPage },
  ],
});
