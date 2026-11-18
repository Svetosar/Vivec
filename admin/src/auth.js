import { ref } from 'vue'
import { getMe, login as loginRequest, logout as logoutRequest } from '@/api'

export const currentUser = ref(null)
let checked = false

export async function loadSession(force = false) {
  if (checked && !force) return currentUser.value
  checked = true
  try {
    currentUser.value = await getMe()
  } catch {
    currentUser.value = null
  }
  return currentUser.value
}

export async function login(credentials) {
  currentUser.value = await loginRequest(credentials)
  checked = true
  return currentUser.value
}

export async function logout() {
  try {
    await logoutRequest()
  } finally {
    currentUser.value = null
    checked = true
  }
}

export function forgetSession() {
  currentUser.value = null
  checked = true
}
