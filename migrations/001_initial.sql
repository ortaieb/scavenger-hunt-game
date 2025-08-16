-- Create enum types (only if they don't already exist)
DO $$ BEGIN
    CREATE TYPE challenge_type AS ENUM ('REC', 'COM', 'RES');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE waypoint_state AS ENUM ('PRESENTED', 'CHECKED_IN', 'VERIFIED');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE audit_event_type AS ENUM (
        'USER_REGISTERED',
        'USER_LOGIN',
        'CHALLENGE_CREATED',
        'CHALLENGE_STARTED',
        'CHALLENGE_ENDED',
        'PARTICIPANT_INVITED',
        'WAYPOINT_CHECKED_IN',
        'WAYPOINT_PROOF_SUBMITTED',
        'WAYPOINT_VERIFIED',
        'LOCATION_UPDATED'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Users table
CREATE TABLE IF NOT EXISTS users (
    user_id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL,
    nickname VARCHAR(100),
    creation_date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- User roles table
CREATE TABLE IF NOT EXISTS user_roles (
    user_id INTEGER REFERENCES users(user_id) ON DELETE CASCADE,
    role_name VARCHAR(50) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (user_id, role_name)
);

-- Challenges table
CREATE TABLE IF NOT EXISTS challenges (
    challenge_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_name VARCHAR(255) NOT NULL,
    challenge_description TEXT,
    challenge_moderator INTEGER REFERENCES users(user_id) ON DELETE CASCADE,
    planned_start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    actual_start_time TIMESTAMP WITH TIME ZONE,
    duration_minutes INTEGER NOT NULL,
    challenge_type challenge_type NOT NULL DEFAULT 'REC',
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Waypoints table
CREATE TABLE IF NOT EXISTS waypoints (
    waypoint_id SERIAL PRIMARY KEY,
    challenge_id UUID REFERENCES challenges(challenge_id) ON DELETE CASCADE,
    waypoint_sequence INTEGER NOT NULL,
    location_lat DOUBLE PRECISION NOT NULL,
    location_lon DOUBLE PRECISION NOT NULL,
    radius_meters DOUBLE PRECISION NOT NULL DEFAULT 50.0,
    waypoint_clue TEXT NOT NULL,
    hints TEXT[],
    waypoint_time_minutes INTEGER DEFAULT -1,
    image_subject TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(challenge_id, waypoint_sequence)
);

-- Challenge participants table
CREATE TABLE IF NOT EXISTS challenge_participants (
    participant_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    challenge_id UUID REFERENCES challenges(challenge_id) ON DELETE CASCADE,
    user_id INTEGER REFERENCES users(user_id) ON DELETE CASCADE,
    participant_nickname VARCHAR(100),
    current_waypoint_id INTEGER REFERENCES waypoints(waypoint_id),
    current_state waypoint_state DEFAULT 'PRESENTED',
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_updated TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(challenge_id, user_id)
);

-- Audit log table
CREATE TABLE IF NOT EXISTS audit_log (
    log_id SERIAL PRIMARY KEY,
    event_time TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    event_type audit_event_type NOT NULL,
    user_id INTEGER REFERENCES users(user_id),
    participant_id UUID REFERENCES challenge_participants(participant_id),
    challenge_id UUID REFERENCES challenges(challenge_id),
    waypoint_id INTEGER REFERENCES waypoints(waypoint_id),
    event_data JSONB,
    outcome VARCHAR(50),
    outcome_payload JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Geolocation log table for participant tracking
CREATE TABLE IF NOT EXISTS geolocation_log (
    log_id SERIAL PRIMARY KEY,
    participant_id UUID REFERENCES challenge_participants(participant_id) ON DELETE CASCADE,
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    location_lat DOUBLE PRECISION NOT NULL,
    location_lon DOUBLE PRECISION NOT NULL,
    accuracy_meters DOUBLE PRECISION
);

-- Create indexes for performance optimization
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);
CREATE INDEX IF NOT EXISTS idx_user_roles_user_id ON user_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_challenges_moderator ON challenges(challenge_moderator);
CREATE INDEX IF NOT EXISTS idx_challenges_start_time ON challenges(planned_start_time);
CREATE INDEX IF NOT EXISTS idx_waypoints_challenge_id ON waypoints(challenge_id);
CREATE INDEX IF NOT EXISTS idx_waypoints_sequence ON waypoints(challenge_id, waypoint_sequence);
CREATE INDEX IF NOT EXISTS idx_challenge_participants_challenge_id ON challenge_participants(challenge_id);
CREATE INDEX IF NOT EXISTS idx_challenge_participants_user_id ON challenge_participants(user_id);
CREATE INDEX IF NOT EXISTS idx_challenge_participants_challenge_user ON challenge_participants(challenge_id, user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_challenge_id ON audit_log(challenge_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_event_time ON audit_log(event_time);
CREATE INDEX IF NOT EXISTS idx_audit_log_event_type ON audit_log(event_type);
CREATE INDEX IF NOT EXISTS idx_geolocation_log_participant ON geolocation_log(participant_id);
CREATE INDEX IF NOT EXISTS idx_geolocation_log_timestamp ON geolocation_log(timestamp);

-- Default roles will be assigned when users are created
-- No default user roles inserted here to avoid foreign key constraint violations