-- Add up migration script here
CREATE TABLE profiles (
    id UUID PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);