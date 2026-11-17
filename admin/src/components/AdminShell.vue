<script setup>
import { useRouter, RouterLink, RouterView } from 'vue-router'
import { currentUser, logout } from '@/auth'

const router = useRouter()

async function signOut() {
  await logout()
  await router.replace({ name: 'login' })
}
</script>

<template>
  <div class="admin-shell">
    <header class="admin-header">
      <RouterLink class="admin-brand" :to="{ name: 'articles' }">Vivec</RouterLink>
      <span class="admin-section">Редакция</span>
      <div class="admin-user">
        <span>{{ currentUser?.username }}</span>
        <button class="button button-quiet" type="button" @click="signOut">Выйти</button>
      </div>
    </header>

    <main class="admin-main">
      <RouterView />
    </main>
  </div>
</template>
