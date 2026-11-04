---
name: identity-service
description: >-
  Operational, architectural, and development guide for identity-service.
  Use when developing, testing, maintaining, debugging, or integrating downstream
  microservices with this Rust-based centralized identity and authentication system
  (Axum, SQLx, PostgreSQL, Ed25519 JWT, JWKS, multi-provider OAuth/OIDC).
---

# Identity Service (`identity-service`) AI Developer & Operator Skill Guide

本指南为任何参与 `identity-service` 开发、调试、测试、运维或下游微服务对接的 AI 代理与工程师提供完整的上下文、设计原则、开发规范与安全红线。

---

## 1. 核心架构哲学与设计理念 (Architecture Philosophy)

```text
               External Identity Providers
           ┌──────────┬──────────┬──────────┐
           │  Google  │    X     │ Supabase │
           └────┬─────┴────┬─────┴────┬─────┘
                │          │          │
                ▼          ▼          ▼
         ┌─────────────────────────────────────┐
         │          identity-service           │
         │                                     │
         │  • Provider Verification Engine     │
         │  • Identity Resolution & Linking    │
         │  • Ed25519 JWT Engine               │
         │  • Refresh Token Lifecycle (CAS)    │
         └──────────────┬──────────────────────┘
                        │
                        ▼
                 PostgreSQL DB
                        │
            JWKS: /.well-known/jwks.json
                        │
         ┌──────────────┴──────────────┐
         ▼ (本地离线纳秒级验签)         ▼ (0 网络 I/O, 0 RPC)
   Backend Service A             Backend Service B
   (Local Cached Ed25519)        (Local Cached Ed25519)
```

### 关键认知（AI 代理必读）：
1. **下游服务验签零网络开销（Zero RPC Overhead）**：
   - 内部 Access Token 采用非对称算法 **Ed25519 (EdDSA)** 签名。
   - 身份中心在 `/.well-known/jwks.json` 公开公钥（RFC 7517 / RFC 8037 标准）。
   - **下游微服务绝不需要在每次请求时调用 identity-service**！下游在启动时拉取并缓存 JWKS，后续所有鉴权均在本地内存纯 CPU 运算（耗时 `< 1μs`），单机并发可达数十万 QPS。
2. **凭证隔离与单向哈希（One-way Refresh Token Hashing）**：
   - 客户端持有高熵随机明文 Refresh Token。
   - 数据库只存储 SHA-256 哈希值，防止数据库只读泄露导致全局会话被劫持。
   - 采用原子 CAS 轮换与单次使用（Single-use Rotation），具备令牌盗用自动阻断（Token Reuse Detection）。

---

## 2. 源码模块速查表 (Component Map)

| 目录/文件 | 核心职责 | 关键类型 / 函数 |
|---|---|---|
| [`src/main.rs`](file:///ssd0/git/identity-service/src/main.rs) | 进程入口、TCP 监听、连接池初始化、优雅停机、后台过期令牌清理任务 | `main`, `shutdown_signal` |
| [`src/config.rs`](file:///ssd0/git/identity-service/src/config.rs) | 强类型环境变量解析与模式判定 | `Config::from_env`, `Environment` |
| [`src/state.rs`](file:///ssd0/git/identity-service/src/state.rs) | Axum 共享应用上下文 | `AppState`, `FromRef` 派生 |
| [`src/error.rs`](file:///ssd0/git/identity-service/src/error.rs) | 统一错误枚举与标准 HTTP JSON 错误响应 | `AppError`, `ErrorResponse`, `IntoResponse` |
| [`src/api/routes.rs`](file:///ssd0/git/identity-service/src/api/routes.rs) | 路由组装、CORS、探针中间件绑定 | `create_router`, `liveness_check`, `readiness_check` |
| [`src/api/auth.rs`](file:///ssd0/git/identity-service/src/api/auth.rs) | 认证端点处理函数 | `exchange`, `refresh`, `revoke` |
| [`src/api/users.rs`](file:///ssd0/git/identity-service/src/api/users.rs) | 用户资料查询、修改、账号绑定 | `get_me`, `update_me`, `get_user_by_id`, `link_identity` |
| [`src/api/middleware.rs`](file:///ssd0/git/identity-service/src/api/middleware.rs) | Axum 请求认证提取器 | `AuthenticatedUser`, `OptionalAuthUser` |
| [`src/api/jwks.rs`](file:///ssd0/git/identity-service/src/api/jwks.rs) | JWKS RFC 7517 格式公钥端点 | `get_jwks` |
| [`src/api/docs.rs`](file:///ssd0/git/identity-service/src/api/docs.rs) | 内置 OpenAPI 3.0 与 Swagger UI | `openapi_spec`, `docs_router` |
| [`src/services/token.rs`](file:///ssd0/git/identity-service/src/services/token.rs) | Ed25519 签名、验签、JWKS 导出 | `JwtEngine::sign`, `verify`, `jwks` |
| [`src/services/session.rs`](file:///ssd0/git/identity-service/src/services/session.rs) | 令牌对签发、原子刷新与单次使用盗用防护 | `SessionService::issue_tokens`, `refresh_tokens`, `revoke_token` |
| [`src/services/identity.rs`](file:///ssd0/git/identity-service/src/services/identity.rs) | 外部身份解析与多账号自动绑定 | `IdentityService::resolve_or_create_user`, `link_identity` |
| [`src/services/user.rs`](file:///ssd0/git/identity-service/src/services/user.rs) | 用户业务实体查询与状态过滤 | `UserService::get_user_by_id`, `update_user` |
| [`src/providers/mod.rs`](file:///ssd0/git/identity-service/src/providers/mod.rs) | 统一验证接口定义与注册表 | `ProviderVerifier`, `ProviderRegistry` |
| [`src/providers/google.rs`](file:///ssd0/git/identity-service/src/providers/google.rs) | Google ID Token 验证适配器 | `GoogleProvider` |
| [`src/providers/x.rs`](file:///ssd0/git/identity-service/src/providers/x.rs) | X (Twitter) OAuth2 验证适配器 | `XProvider` |
| [`src/providers/supabase.rs`](file:///ssd0/git/identity-service/src/providers/supabase.rs) | Supabase JWT 离线 HS256 或 API 验证适配器 | `SupabaseProvider` |
| [`src/providers/mock.rs`](file:///ssd0/git/identity-service/src/providers/mock.rs) | 本地测试/集成测试 Mock 适配器 | `MockProvider`（生产环境绝对禁用） |
| [`src/db/mod.rs`](file:///ssd0/git/identity-service/src/db/mod.rs) | SQLx 连接池、原子 CAS 操作与清理 SQL | `init_pool_with_options`, `rotate_refresh_token_cas`, `cleanup_expired_refresh_tokens` |
| [`migrations/`](file:///ssd0/git/identity-service/migrations/) | PostgreSQL 数据库迁移文件 | `0001_initial_schema.sql` |

---

## 3. 安全红线与开发铁律 (Security Rules & Invariants)

在对代码进行任何修改时，AI 代理必须严格遵守以下六大安全铁律：

> [!CAUTION]
> **铁律 1: 生产模式禁止临时内存私钥**
> 在 `APP_ENV=production` 下，必须通过 `JWT_PRIVATE_KEY_PEM` 或 `JWT_PRIVATE_KEY_PATH` 显式提供 Ed25519 私钥。严禁在生产模式下隐式生成临时密钥，否则会导致实例重启或多副本横向扩展时公私钥分裂，使得下游缓存的 JWKS 验签失败。

> [!CAUTION]
> **铁律 2: 生产环境绝不能启用 `MockProvider`**
> `MockProvider` 允许用任意字符串直接生成有效 JWT，仅供非生产测试使用。`config.enable_mock_provider` 在生产环境下硬编码强制为 `false`，生产模式若收到 `provider: "mock"` 必须返回 400 拒绝。

> [!IMPORTANT]
> **铁律 3: Refresh Token 轮转必须使用原子 CAS 与重放阻断**
> 绝对不能采用“先查后改”的普通 SQL，必须使用 `rotate_refresh_token_cas`。若探测到已吊销（已被刷新过）的旧 Token 再次提交，必须立即判定为令牌窃取，自动调用 `revoke_all_user_refresh_tokens` 阻断该用户家族的全部活跃会话。

> [!IMPORTANT]
> **铁律 4: 用户隐私脱敏保护**
> `/v1/users/{id}` 接口对未认证调用方或非本人调用方，必须返回 [`PublicUserResponse`](file:///ssd0/git/identity-service/src/models/user.rs)（剔除明文 `email`）。只有通过 Bearer Token 验证且查询自身资料时，才允许返回包含 `email` 的完整资料。

> [!IMPORTANT]
> **铁律 5: 外部 Provider 严格验签**
> - Supabase：生产环境必须配置 `SUPABASE_URL` 或 `SUPABASE_JWT_SECRET`（进行 HS256 HMAC 校验），严禁无签名解析。
> - Google：必须校验 `aud == client_id` 且 `email_verified == true`。

---

## 4. 常用操作与开发调试工作流 (Developer Runbook)

### 4.1 编译与格式化
```bash
# 检查语法与编译
cargo check

# 运行完整测试套件
cargo test

# 单独执行生产安全强化专项测试
cargo test --test security_production_test

# 单独执行端到端离线 JWKS 验证集成测试
cargo test --test integration_test
```

### 4.2 本地运行数据库与服务
```bash
# 1. 复制环境变量
cp .env.example .env

# 2. 启动本地 PostgreSQL
docker run -d --name identity-db -p 5432:5432 \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=identity postgres:16-alpine

# 3. 运行服务（自动执行迁移）
cargo run
```

### 4.3 核心探针与诊断端点
- **Liveness 存活探针**: `GET http://localhost:8080/healthz` (200 OK)
- **Readiness 就绪探针**: `GET http://localhost:8080/ready` (真实探测 PostgreSQL `SELECT 1`，DB 故障返回 503)
- **JWKS 公钥端点**: `GET http://localhost:8080/.well-known/jwks.json`
- **交互式文档**: `GET http://localhost:8080/docs` (Swagger UI)

---

## 5. 如何扩展新的第三方登录提供商 (Extension Guide)

当需要接入新的提供商（如 GitHub, Apple, WeCom 等）时，按以下规范操作：

### 步骤 1: 实现 `ProviderVerifier` Trait
在 `src/providers/<provider_name>.rs` 中：
```rust
use crate::error::AppError;
use crate::models::ExternalIdentity;
use crate::providers::{BoxFuture, ProviderVerifier};

pub struct MyProvider {
    // 放入 HTTP client 或配置
}

impl ProviderVerifier for MyProvider {
    fn provider_name(&self) -> &'static str {
        "my_provider"
    }

    fn verify<'a>(&'a self, token: &'a str) -> BoxFuture<'a, Result<ExternalIdentity, AppError>> {
        Box::pin(async move {
            // 1. 校验 Token 签名、有效期、Audience
            // 2. 提取标准 ExternalIdentity
            Ok(ExternalIdentity {
                provider: "my_provider".to_string(),
                provider_user_id: "unique_external_id".to_string(),
                email: Some("user@example.com".to_string()),
                name: Some("User Name".to_string()),
                avatar_url: None,
            })
        })
    }
}
```

### 步骤 2: 注册到 `ProviderRegistry`
在 [`src/providers/mod.rs`](file:///ssd0/git/identity-service/src/providers/mod.rs) 中：
```rust
pub mod my_provider;
pub use my_provider::MyProvider;

impl ProviderRegistry {
    pub fn from_config(config: &Config) -> Self {
        let mut registry = Self::new();
        ...
        registry.register(Arc::new(MyProvider::new(...)));
        registry
    }
}
```

---

## 6. 下游微服务对接标准规范 (Downstream Integration)

AI 在协助编写其他下游微服务时，应指导下游直接采用离线公钥缓存验证，切勿调用 RPC：

### Node.js / TypeScript (`jose`)
```typescript
import { createRemoteJWKSet, jwtVerify } from 'jose';

const JWKS = createRemoteJWKSet(new URL('http://identity-service:8080/.well-known/jwks.json'));

export async function verifyUserToken(token: string) {
  const { payload } = await jwtVerify(token, JWKS, {
    issuer: 'identity-service',
    audience: 'backend-services',
  });
  return {
    userId: payload.sub,
    email: payload.email,
  };
}
```

### Go (`lestrrat-go/jwx`)
```go
ctx := context.Background()
c := jwk.NewCache(ctx)
c.Register("http://identity-service:8080/.well-known/jwks.json", jwk.WithMinRefreshInterval(15*time.Minute))
set, _ := c.Get(ctx, "http://identity-service:8080/.well-known/jwks.json")

token, err := jwt.Parse([]byte(rawToken), jwt.WithKeySet(set), jwt.WithIssuer("identity-service"))
if err == nil {
    userId := token.Subject()
}
```

### Rust (`jsonwebtoken` / `axum`)
下游 Rust 服务可使用 `reqwest` 在启动时缓存公钥，或使用 `ed25519-dalek` 纯 CPU 离线校验。
完整端到端离线校验测试实现可参考 [`tests/integration_test.rs`](file:///ssd0/git/identity-service/tests/integration_test.rs)。
