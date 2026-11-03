use axum::{
    response::{Html, IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

pub fn openapi_spec() -> serde_json::Value {
    json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Centralized Identity Service API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "Production-ready, lightweight identity service featuring external provider delegation, Ed25519 JWT signing, and offline JWKS verification."
        },
        "servers": [
            {
                "url": "/",
                "description": "Current server"
            }
        ],
        "paths": {
            "/healthz": {
                "get": {
                    "summary": "Health Check",
                    "description": "Liveness and readiness health check probe.",
                    "responses": {
                        "200": {
                            "description": "Service is healthy"
                        }
                    }
                }
            },
            "/.well-known/jwks.json": {
                "get": {
                    "summary": "JSON Web Key Set (JWKS)",
                    "description": "RFC 7517 / RFC 8037 JWKS containing public Ed25519 keys for offline JWT verification.",
                    "responses": {
                        "200": {
                            "description": "JWKS public keys"
                        }
                    }
                }
            },
            "/v1/auth/exchange": {
                "post": {
                    "summary": "Exchange External Token",
                    "description": "Verify external provider credential and exchange for internal access and refresh tokens.",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["provider", "token"],
                                    "properties": {
                                        "provider": { "type": "string", "example": "google" },
                                        "token": { "type": "string", "example": "eyJhbGciOi..." }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Token exchange succeeded" },
                        "400": { "description": "Invalid input parameters" },
                        "401": { "description": "External token verification failed" }
                    }
                }
            },
            "/v1/auth/refresh": {
                "post": {
                    "summary": "Refresh Access Token",
                    "description": "Rotate refresh token and issue a fresh internal access token.",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["refresh_token"],
                                    "properties": {
                                        "refresh_token": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Tokens refreshed" },
                        "401": { "description": "Invalid or revoked refresh token" }
                    }
                }
            },
            "/v1/auth/revoke": {
                "post": {
                    "summary": "Revoke Refresh Token",
                    "description": "Invalidate an active refresh token.",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["refresh_token"],
                                    "properties": {
                                        "refresh_token": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Token revoked successfully" }
                    }
                }
            },
            "/v1/users/me": {
                "get": {
                    "summary": "Get Current User Profile",
                    "security": [{ "bearerAuth": [] }],
                    "responses": {
                        "200": { "description": "Current user profile and linked identities" },
                        "401": { "description": "Unauthorized" }
                    }
                },
                "patch": {
                    "summary": "Update Current User Profile",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" },
                                        "avatar_url": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Updated profile" },
                        "401": { "description": "Unauthorized" }
                    }
                }
            },
            "/v1/users/link": {
                "post": {
                    "summary": "Link External Identity",
                    "description": "Attach a new external provider account to the currently authenticated user.",
                    "security": [{ "bearerAuth": [] }],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": ["provider", "token"],
                                    "properties": {
                                        "provider": { "type": "string" },
                                        "token": { "type": "string" }
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Identity linked successfully" },
                        "409": { "description": "Identity already linked to another account" }
                    }
                }
            },
            "/v1/users/{id}": {
                "get": {
                    "summary": "Get User By ID",
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": { "type": "string", "format": "uuid" }
                        }
                    ],
                    "responses": {
                        "200": { "description": "User found" },
                        "404": { "description": "User not found" }
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearerAuth": {
                    "type": "http",
                    "scheme": "bearer",
                    "bearerFormat": "JWT"
                }
            }
        }
    })
}

pub async fn get_openapi_spec() -> Json<serde_json::Value> {
    Json(openapi_spec())
}

pub async fn get_swagger_ui() -> Response {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Identity Service API Documentation</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui.css" />
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.11.0/swagger-ui-bundle.js" crossorigin></script>
  <script>
    window.onload = () => {
      window.ui = SwaggerUIBundle({
        url: '/openapi.json',
        dom_id: '#swagger-ui',
      });
    };
  </script>
</body>
</html>"#;

    Html(html).into_response()
}

pub fn docs_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/openapi.json", get(get_openapi_spec))
        .route("/docs", get(get_swagger_ui))
}
