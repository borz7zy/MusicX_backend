-- Таблица аудио файлов
CREATE TABLE audios (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id    TEXT NOT NULL,                  -- sub из JWT (Keycloak user id)
    title       TEXT NOT NULL,
    description TEXT,
    filename    TEXT NOT NULL,                  -- оригинальное имя файла
    object_key  TEXT NOT NULL UNIQUE,           -- ключ в MinIO (owner_id/uuid.ext)
    size_bytes  BIGINT NOT NULL,
    duration_ms INTEGER,                        -- длительность в мс (опционально)
    is_public   BOOLEAN NOT NULL DEFAULT false, -- разрешено ли в поиске
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_audios_owner_id  ON audios (owner_id);
CREATE INDEX idx_audios_is_public ON audios (is_public);
CREATE INDEX idx_audios_title     ON audios USING gin (to_tsvector('simple', title));

-- Коллекция — чужие аудио, добавленные пользователем
CREATE TABLE audio_collections (
    user_id    TEXT NOT NULL,
    audio_id   UUID NOT NULL REFERENCES audios (id) ON DELETE CASCADE,
    added_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (user_id, audio_id)
);

CREATE INDEX idx_collections_user_id ON audio_collections (user_id);