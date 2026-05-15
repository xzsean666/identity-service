const DEFAULT_API_BASE_URL = "http://127.0.0.1:3000";
const DEFAULT_USERNAME = "alice@example.test";
const MAX_LOG_ENTRIES = 8;

const STORAGE_KEYS = {
  apiBaseUrl: "identity-test.api-base-url",
  username: "identity-test.username",
  accessToken: "identity-test.access-token",
  refreshToken: "identity-test.refresh-token",
};

const state = {
  apiBaseUrl: DEFAULT_API_BASE_URL,
  username: DEFAULT_USERNAME,
  accessToken: "",
  refreshToken: "",
  currentUser: null,
  tokenClaims: null,
  tokenExpiresIn: null,
  latestResponse: null,
  logEntries: [],
};

const elements = {};

document.addEventListener("DOMContentLoaded", initialize);

function initialize() {
  cacheElements();
  hydrateState();
  bindEvents();
  renderAll();
  showStatus("准备就绪。先填 API origin，再注册或登录。", "neutral");
}

function cacheElements() {
  Object.assign(elements, {
    statusMessage: document.getElementById("statusMessage"),
    currentOriginValue: document.getElementById("currentOriginValue"),
    copyOriginButton: document.getElementById("copyOriginButton"),
    allowedOriginCode: document.getElementById("allowedOriginCode"),
    apiBaseUrlInput: document.getElementById("apiBaseUrlInput"),
    apiOriginPreview: document.getElementById("apiOriginPreview"),
    backendAllowedOrigin: document.getElementById("backendAllowedOrigin"),
    connectionWarning: document.getElementById("connectionWarning"),
    settingsForm: document.getElementById("settingsForm"),
    useLocalDefaultButton: document.getElementById("useLocalDefaultButton"),
    resetSessionButton: document.getElementById("resetSessionButton"),
    registerForm: document.getElementById("registerForm"),
    registerUsernameInput: document.getElementById("registerUsernameInput"),
    registerPasswordInput: document.getElementById("registerPasswordInput"),
    loginForm: document.getElementById("loginForm"),
    loginUsernameInput: document.getElementById("loginUsernameInput"),
    loginPasswordInput: document.getElementById("loginPasswordInput"),
    passwordForm: document.getElementById("passwordForm"),
    currentPasswordInput: document.getElementById("currentPasswordInput"),
    newPasswordInput: document.getElementById("newPasswordInput"),
    fetchMeButton: document.getElementById("fetchMeButton"),
    refreshButton: document.getElementById("refreshButton"),
    logoutButton: document.getElementById("logoutButton"),
    accessTokenMeta: document.getElementById("accessTokenMeta"),
    refreshTokenMeta: document.getElementById("refreshTokenMeta"),
    accessTokenOutput: document.getElementById("accessTokenOutput"),
    refreshTokenOutput: document.getElementById("refreshTokenOutput"),
    copyAccessTokenButton: document.getElementById("copyAccessTokenButton"),
    copyRefreshTokenButton: document.getElementById("copyRefreshTokenButton"),
    claimsMeta: document.getElementById("claimsMeta"),
    claimsPreview: document.getElementById("claimsPreview"),
    currentUserPreview: document.getElementById("currentUserPreview"),
    latestAction: document.getElementById("latestAction"),
    latestResponse: document.getElementById("latestResponse"),
    eventLog: document.getElementById("eventLog"),
  });
}

function hydrateState() {
  const queryApiBaseUrl = new URLSearchParams(window.location.search).get("api");
  const storedApiBaseUrl = readStorage(STORAGE_KEYS.apiBaseUrl, DEFAULT_API_BASE_URL);
  const storedUsername = readStorage(STORAGE_KEYS.username, DEFAULT_USERNAME);
  const storedAccessToken = readStorage(STORAGE_KEYS.accessToken, "");
  const storedRefreshToken = readStorage(STORAGE_KEYS.refreshToken, "");
  const storedOrigin = normalizeOrigin(storedApiBaseUrl, DEFAULT_API_BASE_URL);

  state.apiBaseUrl = queryApiBaseUrl
    ? normalizeOrigin(queryApiBaseUrl, storedOrigin)
    : storedOrigin;
  state.username = storedUsername || DEFAULT_USERNAME;
  state.accessToken = storedAccessToken;
  state.refreshToken = storedRefreshToken;

  elements.apiBaseUrlInput.value = state.apiBaseUrl;
  elements.registerUsernameInput.value = state.username;
  elements.loginUsernameInput.value = state.username;
  elements.accessTokenOutput.value = state.accessToken;
  elements.refreshTokenOutput.value = state.refreshToken;
}

function bindEvents() {
  elements.copyOriginButton.addEventListener("click", () => {
    copyText(getPageOrigin(), "页面 origin 已复制。", "success");
  });

  elements.settingsForm.addEventListener("submit", (event) => {
    event.preventDefault();
    try {
      state.apiBaseUrl = normalizeOrigin(elements.apiBaseUrlInput.value);
      saveStorage(STORAGE_KEYS.apiBaseUrl, state.apiBaseUrl);
      elements.apiBaseUrlInput.value = state.apiBaseUrl;
      renderConnection();
      showStatus(`API origin 已保存：${state.apiBaseUrl}`, "success");
    } catch (error) {
      showStatus(error.message, "error");
    }
  });

  elements.useLocalDefaultButton.addEventListener("click", () => {
    state.apiBaseUrl = DEFAULT_API_BASE_URL;
    saveStorage(STORAGE_KEYS.apiBaseUrl, state.apiBaseUrl);
    elements.apiBaseUrlInput.value = state.apiBaseUrl;
    renderConnection();
    showStatus("已切换到本地默认 API origin。", "success");
  });

  elements.resetSessionButton.addEventListener("click", () => {
    state.accessToken = "";
    state.refreshToken = "";
    state.currentUser = null;
    state.tokenClaims = null;
    state.tokenExpiresIn = null;
    state.latestResponse = null;
    state.logEntries = [];
    removeStorage(STORAGE_KEYS.accessToken);
    removeStorage(STORAGE_KEYS.refreshToken);
    renderAll();
    showStatus("当前 session 的 token、user 和日志已清空。", "warning");
  });

  elements.registerForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const username = elements.registerUsernameInput.value.trim();
    const password = elements.registerPasswordInput.value;
    syncUsername(username);
    clearField(elements.registerPasswordInput);
    await submitJson({
      action: "register",
      method: "POST",
      path: "/v1/auth/register",
      body: { username, password },
    });
  });

  elements.loginForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const username = elements.loginUsernameInput.value.trim();
    const password = elements.loginPasswordInput.value;
    syncUsername(username);
    clearField(elements.loginPasswordInput);
    await submitJson({
      action: "login",
      method: "POST",
      path: "/v1/auth/login",
      body: { username, password },
    });
  });

  elements.passwordForm.addEventListener("submit", async (event) => {
    event.preventDefault();
    const currentPassword = elements.currentPasswordInput.value;
    const newPassword = elements.newPasswordInput.value;
    clearField(elements.currentPasswordInput);
    clearField(elements.newPasswordInput);
    await submitJson({
      action: "password change",
      method: "POST",
      path: "/v1/auth/password/change",
      body: { current_password: currentPassword, new_password: newPassword },
      auth: true,
    });
  });

  elements.fetchMeButton.addEventListener("click", async () => {
    await submitJson({
      action: "current user",
      method: "GET",
      path: "/v1/users/me",
      auth: true,
    });
  });

  elements.refreshButton.addEventListener("click", async () => {
    try {
      await submitJson({
        action: "refresh",
        method: "POST",
        path: "/v1/auth/refresh",
        body: { refresh_token: requireRefreshToken() },
      });
    } catch (error) {
      showStatus(error.message, "error");
    }
  });

  elements.logoutButton.addEventListener("click", async () => {
    await submitJson({
      action: "logout",
      method: "POST",
      path: "/v1/auth/logout",
      auth: true,
    });
  });

  elements.copyAccessTokenButton.addEventListener("click", () => {
    copyText(state.accessToken, "Access token 已复制。", "success");
  });

  elements.copyRefreshTokenButton.addEventListener("click", () => {
    copyText(state.refreshToken, "Refresh token 已复制。", "success");
  });
}

function renderAll() {
  renderConnection();
  renderTokens();
  renderClaims();
  renderCurrentUser();
  renderLatestResponse();
  renderLog();
}

function renderConnection() {
  const pageOrigin = getPageOrigin();
  const apiOrigin = state.apiBaseUrl;
  const mixedContentRisk =
    window.location.protocol === "https:" && new URL(apiOrigin).protocol === "http:";

  elements.currentOriginValue.textContent = pageOrigin;
  elements.allowedOriginCode.textContent = pageOrigin;
  elements.apiOriginPreview.textContent = apiOrigin;
  elements.backendAllowedOrigin.textContent = pageOrigin;

  if (mixedContentRisk) {
    elements.connectionWarning.textContent =
      "当前页面是 HTTPS，但 API origin 是 HTTP。浏览器会拦截这些请求。请改成 HTTPS 后端，或者改用本地 wrangler pages dev。";
    elements.connectionWarning.className = "helper helper--warning";
  } else {
    elements.connectionWarning.textContent =
      "后端需要允许上面的 frontend origin 通过 CORS，且 `Authorization` / `Content-Type` 请求头必须可用。";
    elements.connectionWarning.className = "helper";
  }
}

function renderTokens() {
  elements.accessTokenOutput.value = state.accessToken;
  elements.refreshTokenOutput.value = state.refreshToken;

  elements.accessTokenMeta.textContent = state.accessToken
    ? `已保存 access token。${state.tokenExpiresIn ? `expires_in ${state.tokenExpiresIn}s。` : ""}`
    : "尚未获得 access token。";

  elements.refreshTokenMeta.textContent = state.refreshToken
    ? "已保存 refresh token。"
    : "尚未获得 refresh token。";
}

function renderClaims() {
  const claims = decodeJwtClaims(state.accessToken);
  state.tokenClaims = claims;

  if (!claims) {
    elements.claimsMeta.textContent = "登录后会自动解析 access token claims。";
    elements.claimsPreview.textContent = "暂无 claims。";
    return;
  }

  elements.claimsMeta.textContent = [
    `sub ${claims.sub}`,
    `sid ${claims.sid}`,
    `aud ${claims.aud}`,
    claims.exp ? `exp ${formatTimestamp(claims.exp * 1000)}` : null,
  ]
    .filter(Boolean)
    .join(" · ");

  elements.claimsPreview.textContent = JSON.stringify(claims, null, 2);
}

function renderCurrentUser() {
  if (!state.currentUser) {
    elements.currentUserPreview.textContent = "暂无 user。";
    return;
  }

  elements.currentUserPreview.textContent = JSON.stringify(state.currentUser, null, 2);
}

function renderLatestResponse() {
  if (!state.latestResponse) {
    elements.latestAction.textContent = "尚未发起请求。";
    elements.latestResponse.textContent = "暂时没有响应。";
    return;
  }

  elements.latestAction.textContent = `${state.latestResponse.method} ${state.latestResponse.url}`;
  elements.latestResponse.textContent = JSON.stringify(
    {
      at: state.latestResponse.at,
      action: state.latestResponse.action,
      method: state.latestResponse.method,
      url: state.latestResponse.url,
      status: state.latestResponse.status,
      ok: state.latestResponse.ok,
      body: state.latestResponse.body,
    },
    null,
    2,
  );
}

function renderLog() {
  if (!state.logEntries.length) {
    elements.eventLog.innerHTML = '<div class="empty-state">还没有日志。先执行一次注册、登录、刷新或 /me 请求。</div>';
    return;
  }

  elements.eventLog.innerHTML = state.logEntries
    .map((entry) => {
      const statusClass = getStatusClass(entry);
      const summary = truncate(prettyJson(entry.body), 420);

      return `
        <article class="log-entry">
          <div class="log-header">
            <div>
              <p class="log-title">${escapeHtml(entry.action)}</p>
              <p class="log-meta">${escapeHtml(formatTimestamp(entry.at))} · ${escapeHtml(entry.method)} ${escapeHtml(entry.url)}</p>
            </div>
            <span class="status-pill ${statusClass}">${escapeHtml(String(entry.status))}</span>
          </div>
          <pre class="log-summary">${escapeHtml(summary)}</pre>
        </article>
      `;
    })
    .join("");
}

async function submitJson({ action, method, path, body, auth = false }) {
  try {
    const requestUrl = makeApiUrl(path);
    const headers = { Accept: "application/json" };

    if (body !== undefined) {
      headers["Content-Type"] = "application/json";
    }

    if (auth) {
      headers.Authorization = `Bearer ${requireAccessToken()}`;
    }

    let response;
    try {
      response = await fetch(requestUrl, {
        method,
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch (error) {
      const entry = {
        at: new Date().toISOString(),
        action,
        method,
        url: requestUrl,
        status: "network_error",
        ok: false,
        body: { error: error.message },
      };
      pushResponse(entry);
      showStatus(`${action} 网络错误：${error.message}`, "error");
      renderAll();
      return entry;
    }

    const responseBody = await readResponseBody(response);
    const entry = {
      at: new Date().toISOString(),
      action,
      method,
      url: requestUrl,
      status: response.status,
      ok: response.ok,
      body: responseBody,
    };

    if (response.ok) {
      applySuccessfulResponse(action, responseBody);
      showStatus(`${action} 成功 (${response.status})`, "success");
    } else {
      showStatus(`${action} 失败 (${response.status})`, "error");
    }

    pushResponse(entry);
    renderAll();
    return entry;
  } catch (error) {
    const entry = {
      at: new Date().toISOString(),
      action,
      method,
      url: path,
      status: "client_error",
      ok: false,
      body: { error: error.message },
    };
    pushResponse(entry);
    showStatus(`${action} 无法执行：${error.message}`, "error");
    renderAll();
    return entry;
  }
}

function applySuccessfulResponse(action, body) {
  if (!isRecord(body)) {
    return;
  }

  if (isRecord(body.tokens)) {
    state.accessToken = typeof body.tokens.access_token === "string" ? body.tokens.access_token : state.accessToken;
    state.refreshToken = typeof body.tokens.refresh_token === "string" ? body.tokens.refresh_token : state.refreshToken;
    state.tokenExpiresIn = Number.isFinite(body.tokens.expires_in) ? body.tokens.expires_in : null;
    saveStorage(STORAGE_KEYS.accessToken, state.accessToken);
    saveStorage(STORAGE_KEYS.refreshToken, state.refreshToken);
  }

  if (isRecord(body.user)) {
    state.currentUser = body.user;
  } else if (typeof body.internal_user_id === "string") {
    state.currentUser = body;
  }

  if (action === "register" || action === "login") {
    const submittedUsername =
      isRecord(body.user) && typeof body.user.username === "string" ? body.user.username : state.username;
    syncUsername(submittedUsername || state.username);
  }
}

function pushResponse(entry) {
  state.latestResponse = entry;
  state.logEntries = [entry, ...state.logEntries].slice(0, MAX_LOG_ENTRIES);
}

function syncUsername(username) {
  state.username = username || DEFAULT_USERNAME;
  saveStorage(STORAGE_KEYS.username, state.username);
  elements.registerUsernameInput.value = state.username;
  elements.loginUsernameInput.value = state.username;
}

function requireAccessToken() {
  if (!state.accessToken) {
    throw new Error("缺少 access token，请先注册或登录。");
  }

  return state.accessToken;
}

function requireRefreshToken() {
  if (!state.refreshToken) {
    throw new Error("缺少 refresh token，请先注册或登录。");
  }

  return state.refreshToken;
}

function makeApiUrl(path) {
  return new URL(path, state.apiBaseUrl).toString();
}

function normalizeOrigin(value, fallback) {
  if (!value) {
    if (fallback !== undefined) {
      return fallback;
    }

    throw new Error("API origin 不能为空。");
  }

  try {
    const parsed = new URL(value);
    if (!/^https?:$/.test(parsed.protocol)) {
      if (fallback !== undefined) {
        return fallback;
      }

      throw new Error("API origin 只支持 http 或 https。");
    }

    return parsed.origin;
  } catch (error) {
    if (fallback !== undefined) {
      return fallback;
    }

    if (error instanceof Error) {
      throw error;
    }

    throw new Error("API origin 格式不正确。");
  }
}

function getPageOrigin() {
  return window.location.origin && window.location.origin !== "null"
    ? window.location.origin
    : window.location.href;
}

function showStatus(message, tone = "neutral") {
  elements.statusMessage.textContent = message;
  elements.statusMessage.className = `status-message status-message--${tone}`;
}

function copyText(value, successMessage, tone = "success") {
  if (!value) {
    showStatus("没有可复制的内容。", "warning");
    return;
  }

  const clipboard = navigator.clipboard;
  if (clipboard && typeof clipboard.writeText === "function") {
    clipboard
      .writeText(value)
      .then(() => showStatus(successMessage, tone))
      .catch(() => fallbackCopy(value, successMessage, tone));
    return;
  }

  fallbackCopy(value, successMessage, tone);
}

function fallbackCopy(value, successMessage, tone) {
  const textarea = document.createElement("textarea");
  textarea.value = value;
  textarea.setAttribute("readonly", "true");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();

  try {
    document.execCommand("copy");
    showStatus(successMessage, tone);
  } catch {
    showStatus("复制失败，请手动选择文本。", "error");
  } finally {
    document.body.removeChild(textarea);
  }
}

function clearField(input) {
  input.value = "";
}

async function readResponseBody(response) {
  const text = await response.text();
  if (!text) {
    return null;
  }

  try {
    return JSON.parse(text);
  } catch {
    return { raw: text };
  }
}

function decodeJwtClaims(token) {
  if (!token) {
    return null;
  }

  const parts = token.split(".");
  if (parts.length < 2) {
    return null;
  }

  try {
    const payload = parts[1];
    const normalized = payload.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(normalized.length + ((4 - (normalized.length % 4)) % 4), "=");
    const decoded = atob(padded);
    const claims = JSON.parse(decoded);
    return isRecord(claims) ? claims : null;
  } catch {
    return null;
  }
}

function prettyJson(value) {
  if (value === null || value === undefined) {
    return "null";
  }

  if (typeof value === "string") {
    return value;
  }

  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function truncate(value, maxLength) {
  if (value.length <= maxLength) {
    return value;
  }

  return `${value.slice(0, maxLength)}\n…`;
}

function formatTimestamp(input) {
  const date = typeof input === "number" ? new Date(input) : new Date(input);
  return Number.isNaN(date.getTime()) ? "invalid date" : date.toLocaleString();
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function getStatusClass(entry) {
  if (entry.status === "network_error") {
    return "status-error";
  }

  if (!entry.ok) {
    return "status-error";
  }

  if (typeof entry.status === "number" && entry.status >= 200 && entry.status < 300) {
    return "status-ok";
  }

  return "status-warn";
}

function saveStorage(key, value) {
  try {
    window.sessionStorage.setItem(key, value);
  } catch {
    // Ignore storage failures in restrictive browsers.
  }
}

function readStorage(key, fallback) {
  try {
    return window.sessionStorage.getItem(key) ?? fallback;
  } catch {
    return fallback;
  }
}

function removeStorage(key) {
  try {
    window.sessionStorage.removeItem(key);
  } catch {
    // Ignore storage failures in restrictive browsers.
  }
}

function isRecord(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
