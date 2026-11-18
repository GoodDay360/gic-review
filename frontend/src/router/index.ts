import { createRouter, createWebHistory, type RouteLocationNormalized } from 'vue-router'

import { useAuthStore } from '@/stores/auth'
import HomeView from '@/views/HomeView.vue'

declare module 'vue-router' {
  interface RouteMeta {
    requiresStaff?: boolean
    /** Content pages: managers and admins (admins have every manager permission). */
    requiresEditor?: boolean
    requiresAdmin?: boolean
    title?: string
  }
}

/** Where the dashboard starts for the current role. */
function dashboardHome() {
  return '/admin/quizzes'
}

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  scrollBehavior: () => ({ top: 0 }),
  routes: [
    { path: '/', name: 'home', component: HomeView },
    {
      path: '/years/:yearSlug',
      name: 'year',
      component: () => import('@/views/YearView.vue'),
      props: true,
      meta: { title: 'Subjects' },
    },
    {
      path: '/subjects/:subjectSlug/review',
      name: 'review',
      component: () => import('@/views/ReviewView.vue'),
      props: true,
      meta: { title: 'Interactive Review' },
    },
    {
      path: '/subjects/:subjectSlug/quizzes',
      name: 'quizzes',
      component: () => import('@/views/QuizListView.vue'),
      props: true,
      meta: { title: 'Quizzes' },
    },
    {
      path: '/quizzes/:quizId/play',
      name: 'play',
      component: () => import('@/views/QuizPlayView.vue'),
      props: (route: RouteLocationNormalized) => ({
        quizId: route.params.quizId,
        mode: typeof route.query.mode === 'string' ? route.query.mode : undefined,
      }),
      meta: { title: 'Play' },
    },
    {
      path: '/login',
      name: 'login',
      component: () => import('@/views/LoginView.vue'),
      meta: { title: 'Staff login' },
    },
    {
      path: '/admin',
      component: () => import('@/views/admin/AdminLayout.vue'),
      meta: { requiresStaff: true },
      children: [
        { path: '', name: 'admin-home', redirect: () => dashboardHome() },
        {
          path: 'quizzes',
          name: 'admin-quizzes',
          component: () => import('@/views/admin/AdminQuizzesView.vue'),
          meta: { requiresEditor: true, title: 'Quizzes' },
        },
        {
          path: 'quizzes/:id',
          name: 'admin-quiz-edit',
          component: () => import('@/views/admin/AdminQuizEditView.vue'),
          props: true,
          meta: { requiresEditor: true, title: 'Edit quiz' },
        },
        {
          path: 'import',
          name: 'admin-import',
          component: () => import('@/views/admin/AdminImportView.vue'),
          meta: { requiresEditor: true, title: 'Bulk import' },
        },
        {
          path: 'reviews',
          name: 'admin-reviews',
          component: () => import('@/views/admin/AdminReviewsView.vue'),
          meta: { requiresEditor: true, title: 'Interactive reviews' },
        },
        {
          path: 'api-key',
          name: 'manager-api-key',
          component: () => import('@/views/admin/ManagerApiKeyView.vue'),
          meta: { requiresEditor: true, title: 'API key' },
        },
        {
          path: 'api-keys',
          name: 'admin-api-keys',
          component: () => import('@/views/admin/AdminApiKeysView.vue'),
          meta: { requiresAdmin: true, title: 'API keys' },
        },
        {
          path: 'users',
          name: 'admin-users',
          component: () => import('@/views/admin/AdminUsersView.vue'),
          meta: { requiresAdmin: true, title: 'Managers' },
        },
        {
          path: 'years',
          name: 'admin-years',
          component: () => import('@/views/admin/AdminYearsView.vue'),
          meta: { requiresEditor: true, title: 'Years' },
        },
        {
          path: 'subjects',
          name: 'admin-subjects',
          component: () => import('@/views/admin/AdminSubjectsView.vue'),
          meta: { requiresEditor: true, title: 'Subjects' },
        },
      ],
    },
    {
      path: '/:pathMatch(.*)*',
      name: 'not-found',
      component: () => import('@/views/NotFoundView.vue'),
    },
  ],
})

// Frontend guards only hide pages; the backend enforces the same rules on every request.
router.beforeEach(async (to) => {
  const auth = useAuthStore()
  if (to.matched.some((r) => r.meta.requiresStaff || r.meta.requiresAdmin) || to.name === 'login') {
    await auth.fetchMe()
  }
  if (to.name === 'login' && auth.isStaff) return '/admin'
  if (to.matched.some((r) => r.meta.requiresStaff) && !auth.isStaff) {
    return { name: 'login', query: { redirect: to.fullPath } }
  }
  if (to.matched.some((r) => r.meta.requiresAdmin) && !auth.isAdmin) return dashboardHome()
  if (to.matched.some((r) => r.meta.requiresEditor) && !auth.isStaff) return { name: 'login' }
})

router.afterEach((to) => {
  const name = import.meta.env.VITE_APP_NAME || 'GIC Review'
  document.title = to.meta.title ? `${to.meta.title} · ${name}` : name
})

export default router
