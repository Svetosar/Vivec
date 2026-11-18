export class ApiError extends Error {
  constructor(status, message) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

let unauthorizedHandler = null

export function setUnauthorizedHandler(handler) {
  unauthorizedHandler = handler
}

async function request(path, options = {}) {
  const { skipAuthRedirect = false, ...fetchOptions } = options
  const headers = new Headers(fetchOptions.headers)
  headers.set('Accept', 'application/json')

  let body = fetchOptions.body
  if (body && !(body instanceof FormData)) {
    headers.set('Content-Type', 'application/json')
    body = JSON.stringify(body)
  }

  const method = fetchOptions.method || 'GET'
  if (method !== 'GET' && method !== 'HEAD') {
    headers.set('X-Vivec-CSRF', '1')
  }

  const response = await fetch(path, {
    ...fetchOptions,
    method,
    headers,
    body,
    credentials: 'include',
  })

  if (response.status === 204) {
    return null
  }

  const text = await response.text()
  let data = null
  if (text) {
    try {
      data = JSON.parse(text)
    } catch {
      data = null
    }
  }

  if (!response.ok) {
    if (
      response.status === 401 &&
      path !== '/api/admin/login' &&
      !skipAuthRedirect &&
      unauthorizedHandler
    ) {
      unauthorizedHandler()
    }
    const message = data?.error || `Ошибка сервера (${response.status})`
    throw new ApiError(response.status, message)
  }
  return data
}

export function getMe() {
  return request('/api/admin/me', { skipAuthRedirect: true })
}

export function login(credentials) {
  return request('/api/admin/login', {
    method: 'POST',
    body: credentials,
  })
}

export function logout() {
  return request('/api/admin/logout', { method: 'POST' })
}

export function listArticles(params = {}) {
  const query = new URLSearchParams()
  if (params.status && params.status !== 'all') query.set('status', params.status)
  if (params.q) query.set('q', params.q)
  const suffix = query.size ? `?${query}` : ''
  return request(`/api/admin/articles${suffix}`)
}

export function getArticle(id) {
  return request(`/api/admin/articles/${id}`)
}

export function createArticle(article) {
  return request('/api/admin/articles', { method: 'POST', body: article })
}

export function updateArticle(id, article) {
  return request(`/api/admin/articles/${id}`, { method: 'PUT', body: article })
}

export function publishArticle(id) {
  return request(`/api/admin/articles/${id}/publish`, { method: 'POST' })
}

export function unpublishArticle(id) {
  return request(`/api/admin/articles/${id}/unpublish`, { method: 'POST' })
}

export function deleteArticle(id) {
  return request(`/api/admin/articles/${id}`, { method: 'DELETE' })
}

export function uploadImage(file) {
  const body = new FormData()
  body.append('file', file)
  return request('/api/admin/uploads', { method: 'POST', body })
}
