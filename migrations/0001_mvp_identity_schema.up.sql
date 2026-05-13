CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE internal_users (
    internal_user_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    account_status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT internal_users_account_status_check
        CHECK (account_status IN ('active', 'disabled')),
    CONSTRAINT internal_users_updated_after_created_check
        CHECK (updated_at >= created_at)
);

CREATE TABLE external_identities (
    provider_name text NOT NULL,
    provider_subject text NOT NULL,
    internal_user_id uuid NOT NULL
        REFERENCES internal_users (internal_user_id) ON DELETE CASCADE,
    provider_metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (provider_name, provider_subject),
    CONSTRAINT external_identities_provider_name_not_blank_check
        CHECK (length(btrim(provider_name)) > 0),
    CONSTRAINT external_identities_provider_subject_not_blank_check
        CHECK (length(btrim(provider_subject)) > 0),
    CONSTRAINT external_identities_updated_after_created_check
        CHECK (updated_at >= created_at)
);

CREATE INDEX external_identities_internal_user_id_idx
    ON external_identities (internal_user_id);

CREATE TABLE local_credentials (
    credential_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    internal_user_id uuid NOT NULL
        REFERENCES internal_users (internal_user_id) ON DELETE CASCADE,
    username text NOT NULL,
    normalized_username text NOT NULL,
    password_hash text NOT NULL,
    password_hash_algorithm text NOT NULL DEFAULT 'argon2id',
    password_hash_parameters text NOT NULL DEFAULT 'phc_string',
    status text NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT local_credentials_internal_user_id_key
        UNIQUE (internal_user_id),
    CONSTRAINT local_credentials_normalized_username_key
        UNIQUE (normalized_username),
    CONSTRAINT local_credentials_username_not_blank_check
        CHECK (length(btrim(username)) > 0),
    CONSTRAINT local_credentials_normalized_username_not_blank_check
        CHECK (length(btrim(normalized_username)) > 0),
    CONSTRAINT local_credentials_password_hash_not_blank_check
        CHECK (length(btrim(password_hash)) > 0),
    CONSTRAINT local_credentials_password_hash_algorithm_check
        CHECK (length(btrim(password_hash_algorithm)) > 0),
    CONSTRAINT local_credentials_password_hash_parameters_check
        CHECK (length(btrim(password_hash_parameters)) > 0),
    CONSTRAINT local_credentials_status_check
        CHECK (status IN ('active', 'disabled')),
    CONSTRAINT local_credentials_updated_after_created_check
        CHECK (updated_at >= created_at)
);

CREATE TABLE sessions (
    session_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    internal_user_id uuid NOT NULL
        REFERENCES internal_users (internal_user_id) ON DELETE CASCADE,
    provider_name text NOT NULL,
    client_id text NOT NULL DEFAULT '',
    device_metadata jsonb,
    status text NOT NULL DEFAULT 'active',
    issued_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    revoked_at timestamptz,
    CONSTRAINT sessions_session_user_key
        UNIQUE (session_id, internal_user_id),
    CONSTRAINT sessions_provider_name_not_blank_check
        CHECK (length(btrim(provider_name)) > 0),
    CONSTRAINT sessions_status_check
        CHECK (status IN ('active', 'revoked', 'expired')),
    CONSTRAINT sessions_expires_after_issued_check
        CHECK (expires_at > issued_at),
    CONSTRAINT sessions_revoked_at_check
        CHECK (
            (status = 'revoked' AND revoked_at IS NOT NULL)
            OR (status <> 'revoked')
        )
);

CREATE INDEX sessions_internal_user_id_status_idx
    ON sessions (internal_user_id, status);

CREATE INDEX sessions_expires_at_idx
    ON sessions (expires_at);

CREATE TABLE refresh_token_records (
    refresh_token_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id uuid NOT NULL,
    internal_user_id uuid NOT NULL,
    token_family_id uuid NOT NULL,
    token_hash text NOT NULL,
    status text NOT NULL DEFAULT 'active',
    issued_at timestamptz NOT NULL DEFAULT now(),
    expires_at timestamptz NOT NULL,
    consumed_at timestamptz,
    revoked_at timestamptz,
    replaced_by_refresh_token_id uuid
        REFERENCES refresh_token_records (refresh_token_id) ON DELETE SET NULL,
    CONSTRAINT refresh_token_records_session_user_fkey
        FOREIGN KEY (session_id, internal_user_id)
        REFERENCES sessions (session_id, internal_user_id) ON DELETE CASCADE,
    CONSTRAINT refresh_token_records_token_hash_key
        UNIQUE (token_hash),
    CONSTRAINT refresh_token_records_token_hash_not_blank_check
        CHECK (length(btrim(token_hash)) > 0),
    CONSTRAINT refresh_token_records_status_check
        CHECK (status IN ('active', 'consumed', 'revoked', 'reused', 'expired')),
    CONSTRAINT refresh_token_records_expires_after_issued_check
        CHECK (expires_at > issued_at),
    CONSTRAINT refresh_token_records_consumed_at_check
        CHECK (
            (status = 'consumed' AND consumed_at IS NOT NULL)
            OR (status <> 'consumed')
        ),
    CONSTRAINT refresh_token_records_revoked_at_check
        CHECK (
            (status IN ('revoked', 'reused') AND revoked_at IS NOT NULL)
            OR (status NOT IN ('revoked', 'reused'))
        )
);

CREATE UNIQUE INDEX refresh_token_records_one_active_per_family_idx
    ON refresh_token_records (token_family_id)
    WHERE status = 'active';

CREATE INDEX refresh_token_records_session_id_idx
    ON refresh_token_records (session_id);

CREATE INDEX refresh_token_records_internal_user_id_status_idx
    ON refresh_token_records (internal_user_id, status);

CREATE INDEX refresh_token_records_family_issued_at_idx
    ON refresh_token_records (token_family_id, issued_at);

CREATE INDEX refresh_token_records_expires_at_idx
    ON refresh_token_records (expires_at);
