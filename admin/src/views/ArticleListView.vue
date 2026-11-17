<script setup>
import { onMounted, reactive, ref } from 'vue'
import { RouterLink } from 'vue-router'
import { deleteArticle, listArticles, publishArticle, unpublishArticle } from '@/api'

const items = ref([])
const loading = ref(true)
const error = ref('')
const filters = reactive({ status: 'all', q: '' })

async function load() {
  loading.value = true
  error.value = ''
  try {
    const result = await listArticles(filters)
    items.value = result.items
  } catch (requestError) {
    error.value = requestError.message
  } finally {
    loading.value = false
  }
}

async function togglePublished(article) {
  error.value = ''
  try {
    if (article.published) await unpublishArticle(article.id)
    else await publishArticle(article.id)
    await load()
  } catch (requestError) {
    error.value = requestError.message
  }
}

async function remove(article) {
  if (!window.confirm(`Удалить «${article.title}»? Это действие необратимо.`)) return
  error.value = ''
  try {
    await deleteArticle(article.id)
    await load()
  } catch (requestError) {
    error.value = requestError.message
  }
}

onMounted(load)
</script>

<template>
  <section class="page-heading">
    <div>
      <p class="eyebrow">Материалы</p>
      <h1>Статьи</h1>
    </div>
    <RouterLink class="button button-primary" :to="{ name: 'article-new' }">
      Новая статья
    </RouterLink>
  </section>

  <form class="filters" @submit.prevent="load">
    <label>
      Поиск
      <input v-model.trim="filters.q" type="search" placeholder="Заголовок или автор" />
    </label>
    <label>
      Статус
      <select v-model="filters.status" @change="load">
        <option value="all">Все</option>
        <option value="draft">Черновики</option>
        <option value="published">Опубликованные</option>
      </select>
    </label>
    <button class="button" type="submit">Найти</button>
  </form>

  <p v-if="error" class="notice notice-error">{{ error }}</p>
  <p v-if="loading" class="muted">Загрузка…</p>

  <div v-else-if="items.length === 0" class="empty-state">
    <h2>Материалов пока нет</h2>
    <p>Создайте первый черновик.</p>
  </div>

  <div v-else class="article-table-wrap">
    <table class="article-table">
      <thead>
        <tr>
          <th>Материал</th>
          <th>Статус</th>
          <th>Дата</th>
          <th><span class="sr-only">Действия</span></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="article in items" :key="article.id">
          <td>
            <RouterLink
              class="article-title-link"
              :to="{ name: 'article-edit', params: { id: article.id } }"
            >
              {{ article.title }}
            </RouterLink>
            <small>{{ article.rubric }} · {{ article.author || 'Без автора' }}</small>
          </td>
          <td>
            <span class="status" :class="article.published ? 'status-live' : 'status-draft'">
              {{ article.published ? 'Опубликовано' : 'Черновик' }}
            </span>
          </td>
          <td>{{ article.published_at || article.created_at.slice(0, 10) }}</td>
          <td class="row-actions">
            <a
              v-if="article.published"
              class="text-action"
              :href="`/${article.slug}`"
              target="_blank"
              rel="noopener"
            >
              Открыть
            </a>
            <button class="text-action" type="button" @click="togglePublished(article)">
              {{ article.published ? 'Снять' : 'Опубликовать' }}
            </button>
            <button class="text-action text-danger" type="button" @click="remove(article)">
              Удалить
            </button>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
