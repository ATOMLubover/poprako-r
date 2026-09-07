CREATE TABLE IF NOT EXISTS "t_chapter_artwork" (
    "f_id"                         TEXT        PRIMARY KEY,
    "f_version"                    BIGINT      NOT NULL,
    "f_key"                        TEXT,
    "f_is_uploaded"                BOOLEAN,
    "f_hash"                       BYTEA,
    "f_ext"                        TEXT,
    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
