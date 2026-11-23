const API_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8080";

export function shortUrl(code: string) {
  return `${API_URL.replace(/\/$/, "")}/${code}`;
}

export interface AuthResult {
  token: string;
  email: string;
}

export interface ShortUrl {
  id: string;
  original_url: string;
  short_code: string;
  created_at: string;
  expires_at: string | null;
  is_active: boolean;
  clicks: number;
}

export interface CreatedUrl {
  id: string;
  original_url: string;
  short_code: string;
  short_url: string;
  created_at: string;
  expires_at: string | null;
}

export interface Analytics {
  short_code: string;
  total_clicks: number;
  daily_clicks: Array<{ day: string; clicks: number }>;
}

async function request<T>(
  path: string,
  token?: string,
  options: RequestInit = {},
): Promise<T> {
  const headers = new Headers(options.headers);
  if (options.body) headers.set("Content-Type", "application/json");
  if (token) headers.set("Authorization", `Bearer ${token}`);

  const response = await fetch(`${API_URL}${path}`, { ...options, headers });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as
      | { error?: string }
      | null;
    throw new Error(body?.error ?? `Request failed (${response.status})`);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export const api = {
  authenticate(
    action: "register" | "login",
    email: string,
    password: string,
  ) {
    return request<AuthResult>(`/api/auth/${action}`, undefined, {
      method: "POST",
      body: JSON.stringify({ email, password }),
    });
  },
  listUrls(token: string) {
    return request<ShortUrl[]>("/api/urls", token);
  },
  createUrl(
    token: string,
    values: { original_url: string; custom_alias?: string; expires_at?: string },
  ) {
    return request<CreatedUrl>("/api/urls", token, {
      method: "POST",
      body: JSON.stringify(values),
    });
  },
  deleteUrl(token: string, code: string) {
    return request<void>(`/api/urls/${encodeURIComponent(code)}`, token, {
      method: "DELETE",
    });
  },
  analytics(token: string, code: string) {
    return request<Analytics>(
      `/api/urls/${encodeURIComponent(code)}/analytics`,
      token,
    );
  },
};
