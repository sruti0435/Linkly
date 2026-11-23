SELECT
    'CREATE ROLE rusturl LOGIN PASSWORD ' || quote_literal('rusturl')
WHERE NOT EXISTS (
    SELECT 1 FROM pg_roles WHERE rolname = 'rusturl'
)
\gexec

ALTER ROLE rusturl WITH LOGIN PASSWORD 'rusturl';

SELECT 'CREATE DATABASE rusturl OWNER rusturl'
WHERE NOT EXISTS (
    SELECT 1 FROM pg_database WHERE datname = 'rusturl'
)
\gexec

ALTER DATABASE rusturl OWNER TO rusturl;
