-- PostgreSQL 接続確認用の最小セットアップ
-- 例:
--   psql -U postgres -f ops/sql/bootstrap_postgres.sql

DO
$$
BEGIN
  IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'iot_user') THEN
    CREATE ROLE iot_user LOGIN PASSWORD 'iot_password';
  END IF;
END
$$;

SELECT 'CREATE DATABASE iot_hub OWNER iot_user'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'iot_hub')
\gexec

\connect iot_hub

CREATE TABLE IF NOT EXISTS sensors (
  id serial PRIMARY KEY,
  temperature real NOT NULL,
  pressure double precision NOT NULL
);

CREATE TABLE IF NOT EXISTS system_status (
  id serial PRIMARY KEY,
  status text NOT NULL
);

TRUNCATE TABLE sensors RESTART IDENTITY;
TRUNCATE TABLE system_status RESTART IDENTITY;

INSERT INTO sensors (temperature, pressure)
VALUES (25.4, 0.82);

INSERT INTO system_status (status)
VALUES ('RUNNING');

GRANT CONNECT ON DATABASE iot_hub TO iot_user;
GRANT USAGE ON SCHEMA public TO iot_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO iot_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO iot_user;
