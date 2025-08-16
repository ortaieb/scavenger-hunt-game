-- Create test database
CREATE DATABASE scavenger_test;
GRANT ALL PRIVILEGES ON DATABASE scavenger_test TO scavenger_user;

-- Grant schema permissions
GRANT ALL ON SCHEMA public TO scavenger_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO scavenger_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO scavenger_user;