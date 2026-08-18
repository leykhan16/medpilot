CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('patient', 'clinician', 'admin')),
    name          TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE patient_profiles (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id    UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    dob        DATE,
    gender     TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE cases (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    patient_id    UUID NOT NULL REFERENCES patient_profiles(id) ON DELETE CASCADE,
    status        TEXT NOT NULL DEFAULT 'open'
                  CHECK (status IN ('open', 'analyzed', 'booked', 'closed')),
    urgency_score SMALLINT CHECK (urgency_score BETWEEN 0 AND 100),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE case_messages (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    case_id    UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    sender     TEXT NOT NULL CHECK (sender IN ('patient', 'ai')),
    content    TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE ai_analyses (
    id                 UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    case_id            UUID NOT NULL UNIQUE REFERENCES cases(id) ON DELETE CASCADE,
    level_of_care      TEXT NOT NULL,
    possible_conditions JSONB NOT NULL,
    confidence         REAL NOT NULL CHECK (confidence BETWEEN 0 AND 1),
    model_used         TEXT NOT NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE appointments (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    case_id       UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    clinician_id  UUID REFERENCES users(id) ON DELETE SET NULL,
    scheduled_at  TIMESTAMPTZ NOT NULL,
    status        TEXT NOT NULL DEFAULT 'scheduled'
                  CHECK (status IN ('scheduled', 'completed', 'cancelled')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE alerts (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    case_id      UUID NOT NULL REFERENCES cases(id) ON DELETE CASCADE,
    reason       TEXT NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT false,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_cases_status ON cases(status);
CREATE INDEX idx_cases_patient ON cases(patient_id);
CREATE INDEX idx_case_messages_case ON case_messages(case_id, created_at);
CREATE INDEX idx_alerts_unacknowledged ON alerts(acknowledged) WHERE acknowledged = false;
