import { API_BASE_URL } from '@/config'

/** Error thrown for any non-2xx API response (`{ error: { code, message } }`). */
export class ApiError extends Error {
  constructor(
    public status: number,
    public code: string,
    message: string,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

async function toApiError(status: number, text: string): Promise<ApiError> {
  try {
    const body = JSON.parse(text)
    if (body?.error?.message)
      return new ApiError(status, body.error.code ?? 'error', body.error.message)
  } catch {
    /* not JSON */
  }
  return new ApiError(status, 'http_error', `Request failed (HTTP ${status})`)
}

export async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  init: RequestInit = {},
): Promise<T> {
  const headers: Record<string, string> = { Accept: 'application/json' }
  let payload: BodyInit | undefined
  if (body instanceof FormData) {
    payload = body
  } else if (body !== undefined) {
    headers['Content-Type'] = 'application/json'
    payload = JSON.stringify(body)
  }

  let res: Response
  try {
    res = await fetch(`${API_BASE_URL}${path}`, {
      method,
      body: payload,
      headers,
      // Send the httpOnly session cookie.
      credentials: 'include',
      ...init,
    })
  } catch {
    throw new ApiError(0, 'network_error', 'Cannot reach the server. Check your connection.')
  }

  const text = await res.text()
  if (!res.ok) throw await toApiError(res.status, text)
  return (text ? JSON.parse(text) : undefined) as T
}

export const api = {
  get: <T>(path: string) => request<T>('GET', path),
  post: <T>(path: string, body?: unknown) => request<T>('POST', path, body),
  put: <T>(path: string, body?: unknown) => request<T>('PUT', path, body),
  patch: <T>(path: string, body?: unknown) => request<T>('PATCH', path, body),
  delete: <T = void>(path: string) => request<T>('DELETE', path),
}

/** Multipart upload with progress (fetch has no upload progress events). */
export function uploadWithProgress<T>(
  path: string,
  form: FormData,
  onProgress: (fraction: number) => void,
  method = 'POST',
): Promise<T> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open(method, `${API_BASE_URL}${path}`)
    xhr.withCredentials = true
    xhr.setRequestHeader('Accept', 'application/json')
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) onProgress(e.loaded / e.total)
    }
    xhr.onload = async () => {
      if (xhr.status >= 200 && xhr.status < 300) {
        onProgress(1)
        resolve((xhr.responseText ? JSON.parse(xhr.responseText) : undefined) as T)
      } else {
        reject(await toApiError(xhr.status, xhr.responseText))
      }
    }
    xhr.onerror = () =>
      reject(new ApiError(0, 'network_error', 'Upload failed: cannot reach the server.'))
    xhr.send(form)
  })
}

export function errorMessage(err: unknown): string {
  if (err instanceof ApiError) return err.message
  if (err instanceof Error) return err.message
  return 'Something went wrong'
}
