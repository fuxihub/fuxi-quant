import { createRouter, createWebHistory } from 'vue-router'

const routes = [
    {
        path: '/',
        redirect: '/agent',
    },
    {
        path: '/agent',
        name: 'Agent',
        component: () => import('@/pages/AgentPage.vue'),
    },
    {
        path: '/strategy',
        name: 'Strategy',
        component: () => import('@/pages/StrategyPage.vue'),
    },
    {
        path: '/backtest',
        name: 'Backtest',
        component: () => import('@/pages/BacktestPage.vue'),
    },
    {
        path: '/data',
        name: 'Data',
        component: () => import('@/pages/DataPage.vue'),
    },
    {
        path: '/live',
        name: 'Live',
        component: () => import('@/pages/LivePage.vue'),
    },
    {
        path: '/settings',
        name: 'Settings',
        component: () => import('@/pages/SettingsPage.vue'),
    },
]

const router = createRouter({
    history: createWebHistory(),
    routes,
})

export default router
