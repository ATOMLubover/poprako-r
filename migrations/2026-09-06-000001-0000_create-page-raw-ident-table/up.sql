CREATE TABLE IF NOT EXISTS "t_page_raw_ident" (
    "f_page_id"                    TEXT        PRIMARY KEY REFERENCES "t_page" ("f_id") ON DELETE CASCADE,
    "f_raw_ident"                  TEXT        NOT NULL,

    "f_created_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "f_updated_at"                 TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
