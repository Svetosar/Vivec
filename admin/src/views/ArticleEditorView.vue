<script setup>
import { computed, onMounted, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { MdEditor } from 'md-editor-v3'
import 'md-editor-v3/lib/style.css'
import {
  createArticle,
  deleteArticle,
  getArticle,
  publishArticle,
  unpublishArticle,
  updateArticle,
  uploadImage,
} from '@/api'

const rubrics = ['Школа', 'Спорт', 'Культура', 'Мнения']
const route = useRoute()
const router = useRouter()
const articleId = computed(() => (route.params.id ? Number(route.params.id) : null))
const article = reactive({
  id: null,
  title: '',
  slug: '',
  rubric: 'Школа',
  author: '',
  lead: '',
  body_md: '',
  published: false,
  published_at: null,
})
const loading = ref(Boolean(articleId.value))
const busy = ref(false)
const error = ref('')
const saved = ref('')

function payload() {
  return {
    title: article.title,
    slug: article.slug,
    rubric: article.rubric,
    author: article.author,
    lead: article.lead,
    body_md: article.body_md,
  }
}

function replaceArticle(data) {
  Object.assign(article, data)
}

async function save() {
  error.value = ''
  saved.value = ''
  busy.value = true
  try {
    const result = articleId.value
      ? await updateArticle(articleId.value, payload())
      : await createArticle(payload())
    replaceArticle(result)
    if (!articleId.value) {
      await router.replace({ name: 'article-edit', params: { id: result.id } })
    }
    saved.value = 'Сохранено'
  } catch (requestError) {
    error.value = requestError.message
  } finally {
    busy.value = false
  }
}

async function togglePublish() {
  if (!articleId.value) {
    error.value = 'Сначала сохраните статью'
    return
  }
  error.value = ''
  busy.value = true
  try {
    const result = article.published
      ? await unpublishArticle(articleId.value)
      : await publishArticle(articleId.value)
    replaceArticle(result)
    saved.value = result.published ? 'Опубликовано' : 'Снято с публикации'
  } catch (requestError) {
    error.value = requestError.message
  } finally {
    busy.value = false
  }
}

async function remove() {
  if (!articleId.value || !window.confirm('Удалить статью без возможности восстановления?')) return
  busy.value = true
  try {
    await deleteArticle(articleId.value)
    await router.replace({ name: 'articles' })
  } catch (requestError) {
    error.value = requestError.message
  } finally {
    busy.value = false
  }
}

async function upload(files, callback) {
  try {
    const urls = []
    for (const file of files) {
      const uploaded = await uploadImage(file)
      urls.push(uploaded.url)
    }
    callback(urls)
  } catch (requestError) {
    error.value = requestError.message
    callback([])
  }
}

onMounted(async () => {
  if (!articleId.value) return
  try {
    replaceArticle(await getArticle(articleId.value))
  } catch (requestError) {
    error.value = requestError.message
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <section class="page-heading editor-heading">
    <div>
      <p class="eyebrow">{{ articleId ? 'Редактирование' : 'Новый материал' }}</p>
      <h1>{{ article.title || 'Без заголовка' }}</h1>
    </div>
    <div class="editor-actions">
      <button class="button" type="button" :disabled="busy" @click="save">Сохранить</button>
      <button class="button button-primary" type="button" :disabled="busy" @click="togglePublish">
        {{ article.published ? 'Снять с публикации' : 'Опубликовать' }}
      </button>
    </div>
  </section>

  <p v-if="loading" class="muted">Загрузка статьи…</p>
  <template v-else>
    <p v-if="error" class="notice notice-error">{{ error }}</p>
    <p v-if="saved" class="notice notice-success">{{ saved }}</p>

    <form class="editor-form" @submit.prevent="save">
      <div class="field-grid">
        <label class="field-wide">
          Заголовок
          <input v-model="article.title" maxlength="200" required />
        </label>
        <label>
          Slug
          <input v-model="article.slug" maxlength="120" placeholder="Создастся из заголовка" />
        </label>
        <label>
          Рубрика
          <select v-model="article.rubric">
            <option v-for="rubric in rubrics" :key="rubric" :value="rubric">{{ rubric }}</option>
          </select>
        </label>
        <label>
          Автор
          <input v-model="article.author" maxlength="100" />
        </label>
        <label class="field-wide">
          Лид
          <textarea v-model="article.lead" maxlength="500" rows="3"></textarea>
        </label>
      </div>

      <div class="markdown-field">
        <span class="field-label">Текст статьи</span>
        <MdEditor
          v-model="article.body_md"
          language="en"
          :no-mermaid="true"
          :no-katex="true"
          :no-highlight="true"
          :no-echarts="true"
          :no-prettier="true"
          :on-upload-img="upload"
        />
      </div>

      <div class="danger-zone">
        <div>
          <strong>Удаление</strong>
          <p>Материал исчезнет из базы без возможности отмены.</p>
        </div>
        <button class="button button-danger" type="button" :disabled="busy" @click="remove">
          Удалить статью
        </button>
      </div>
    </form>
  </template>
</template>
