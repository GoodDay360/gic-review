import { createRouter, createWebHistory, type RouteLocationNormalized } from 'vue-router'

import { useAuthStore } from '@/stores/auth'
import HomeView from '@/views/HomeView.vue'

declare module 'vue-router' {
  interface RouteMeta {
    requiresStaff?: boolean
    requiresAdmin?: boolean
    title?: string
  }
}

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  scrollBehavior: () => ({ top: 0 }),
  routes: [
    { path: '/', name: 'home', component: HomeView },
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
        { path: '', redirect: '/admin/quizzes' },
        {
          path: 'quizzes',
          name: 'admin-quizzes',
          component: () => import('@/views/admin/AdminQuizzesView.vue'),
          meta: { title: 'Quizzes' },
        },
        {
          path: 'quizzes/:id',
          name: 'admin-quiz-edit',
          component: () => import('@/views/admin/AdminQuizEditView.vue'),
          props: true,
          meta: { title: 'Edit quiz' },
        },
        {
          path: 'import',
          name: 'admin-import',
          component: () => import('@/views/admin/AdminImportView.vue'),
          meta: { title: 'Bulk import' },
        },
        {
          path: 'reviews',
          name: 'admin-reviews',
          component: () => import('@/views/admin/AdminReviewsView.vue'),
          meta: { title: 'Interactive reviews' },
        },
        {
          path: 'users',
          name: 'admin-users',
          component: () => import('@/views/admin/AdminUsersView.vue'),
          meta: { requiresAdmin: true, title: 'Managers' },
        },
        {
          path: 'subjects',
          name: 'admin-subjects',
          component: () => import('@/views/admin/AdminSubjectsView.vue'),
          meta: { requiresAdmin: true, title: 'Subjects' },
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
  if (to.matched.some((r) => r.meta.requiresAdmin) && !auth.isAdmin) {
    return { name: 'admin-quizzes' }
  }
})

router.afterEach((to) => {
  const name = import.meta.env.VITE_APP_NAME || 'GIC Review'
  document.title = to.meta.title ? `${to.meta.title} · ${name}` : name
})

export default router
