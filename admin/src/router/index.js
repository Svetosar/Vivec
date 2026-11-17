import { createRouter, createWebHistory } from 'vue-router'
import { loadSession } from '@/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('@/views/LoginView.vue'),
      meta: { public: true },
    },
    {
      path: '/',
      component: () => import('@/components/AdminShell.vue'),
      children: [
        { path: '', redirect: { name: 'articles' } },
        {
          path: 'articles',
          name: 'articles',
          component: () => import('@/views/ArticleListView.vue'),
        },
        {
          path: 'articles/new',
          name: 'article-new',
          component: () => import('@/views/ArticleEditorView.vue'),
        },
        {
          path: 'articles/:id/edit',
          name: 'article-edit',
          component: () => import('@/views/ArticleEditorView.vue'),
        },
      ],
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: { name: 'articles' },
    },
  ],
})

router.beforeEach(async (to) => {
  const user = await loadSession()
  if (to.meta.public) {
    return user ? { name: 'articles' } : true
  }
  if (!user) {
    return { name: 'login', query: { next: to.fullPath } }
  }
  return true
})

export default router
