<script setup>
import { reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { login } from '@/auth'

const router = useRouter()
const route = useRoute()
const form = reactive({ username: '', password: '' })
const error = ref('')
const busy = ref(false)

async function submit() {
  error.value = ''
  busy.value = true
  try {
    await login(form)
    const next = typeof route.query.next === 'string' ? route.query.next : '/articles'
    await router.replace(next)
  } catch (requestError) {
    error.value = requestError.message
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <main class="login-page">
    <form class="login-card" @submit.prevent="submit">
      <p class="eyebrow">Vivec · редакция</p>
      <h1>Вход</h1>
      <p class="muted">Управление статьями школьной газеты.</p>

      <label>
        Пользователь
        <input v-model="form.username" name="username" autocomplete="username" required />
      </label>

      <label>
        Пароль
        <input
          v-model="form.password"
          name="password"
          type="password"
          autocomplete="current-password"
          required
        />
      </label>

      <p v-if="error" class="notice notice-error">{{ error }}</p>
      <button class="button button-primary" type="submit" :disabled="busy">
        {{ busy ? 'Проверяем…' : 'Войти' }}
      </button>
    </form>
  </main>
</template>
