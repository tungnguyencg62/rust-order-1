#!/bin/bash
set -e

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" <<EOSQL
    CREATE USER supabase_auth_admin NOINHERIT CREATEROLE LOGIN NOREPLICATION PASSWORD '${AUTH_DB_PASSWORD}';
    CREATE SCHEMA IF NOT EXISTS auth AUTHORIZATION supabase_auth_admin;
    GRANT CREATE ON DATABASE "$POSTGRES_DB" TO supabase_auth_admin;
    ALTER USER supabase_auth_admin SET search_path = 'auth';

    CREATE USER app_user LOGIN PASSWORD '${APP_DB_PASSWORD}';
    CREATE DATABASE app OWNER app_user;
EOSQL