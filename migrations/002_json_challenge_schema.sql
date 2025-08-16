-- Migration: Replace challenges and waypoints tables with JSON-based temporal storage
-- GitHub Issue #6: Replace store of challenge with a json record

-- Create sequence for challenge_id
CREATE SEQUENCE IF NOT EXISTS challenge_id_seq;

-- Create sequence for challenge_version_id
CREATE SEQUENCE IF NOT EXISTS challenge_version_id_seq;

-- Create new temporal challenges table with JSON storage
CREATE TABLE IF NOT EXISTS temporal_challenges (
    challenge_id INTEGER DEFAULT nextval('challenge_id_seq') NOT NULL,
    challenge_version_id INTEGER DEFAULT nextval('challenge_version_id_seq') PRIMARY KEY,
    challenge_name TEXT NOT NULL,
    planned_start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    challenge JSONB NOT NULL,
    start_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    end_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_id ON temporal_challenges(challenge_id);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_temporal ON temporal_challenges(challenge_id, start_at, end_at);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_name ON temporal_challenges(challenge_name);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_start_time ON temporal_challenges(planned_start_time);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_json ON temporal_challenges USING GIN(challenge);

-- Create view for current (active) challenges
CREATE OR REPLACE VIEW current_challenges AS
SELECT *
FROM temporal_challenges
WHERE end_at IS NULL;

-- Data migration: Move existing challenges and waypoints to JSON format
-- Note: This assumes the old tables exist and need to be migrated
DO $$
DECLARE
    challenge_rec RECORD;
    waypoints_json JSONB;
    challenge_json JSONB;
BEGIN
    -- Only proceed if old tables exist
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'challenges') THEN
        FOR challenge_rec IN 
            SELECT * FROM challenges ORDER BY created_at
        LOOP
            -- Build waypoints JSON
            SELECT COALESCE(
                jsonb_agg(
                    jsonb_build_object(
                        'waypoint_id', waypoint_id,
                        'waypoint_sequence', waypoint_sequence,
                        'location', jsonb_build_object('lat', location_lat, 'lon', location_lon),
                        'radius_meters', radius_meters,
                        'waypoint_clue', waypoint_clue,
                        'hints', CASE WHEN hints IS NOT NULL THEN to_jsonb(hints) ELSE '[]'::jsonb END,
                        'waypoint_time_minutes', waypoint_time_minutes,
                        'image_subject', image_subject,
                        'created_at', created_at
                    ) ORDER BY waypoint_sequence
                ), 
                '[]'::jsonb
            ) INTO waypoints_json
            FROM waypoints 
            WHERE challenge_id = challenge_rec.challenge_id;
            
            -- Build complete challenge JSON
            challenge_json := jsonb_build_object(
                'challenge_id', challenge_rec.challenge_id,
                'challenge_description', challenge_rec.challenge_description,
                'challenge_moderator', challenge_rec.challenge_moderator,
                'actual_start_time', challenge_rec.actual_start_time,
                'duration_minutes', challenge_rec.duration_minutes,
                'challenge_type', challenge_rec.challenge_type,
                'active', challenge_rec.active,
                'waypoints', waypoints_json,
                'metadata', jsonb_build_object(
                    'created_at', challenge_rec.created_at,
                    'updated_at', challenge_rec.updated_at,
                    'migrated_from_relational', true
                )
            );
            
            -- Insert into new temporal table
            INSERT INTO temporal_challenges (
                challenge_name,
                planned_start_time,
                challenge,
                start_at,
                created_at,
                updated_at
            ) VALUES (
                challenge_rec.challenge_name,
                challenge_rec.planned_start_time,
                challenge_json,
                challenge_rec.created_at,
                challenge_rec.created_at,
                challenge_rec.updated_at
            );
        END LOOP;
    END IF;
END $$;

-- Update challenge_participants to reference challenge_id instead of UUID
-- Note: This assumes we need to update the foreign key relationship
DO $$
BEGIN
    -- Only proceed if old table exists and needs updating
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'challenge_participants') THEN
        -- Add new column for integer challenge_id
        IF NOT EXISTS (SELECT FROM information_schema.columns 
                      WHERE table_name = 'challenge_participants' 
                      AND column_name = 'temporal_challenge_id') THEN
            ALTER TABLE challenge_participants ADD COLUMN temporal_challenge_id INTEGER;
            
            -- Populate the new column based on existing UUID mappings
            UPDATE challenge_participants cp
            SET temporal_challenge_id = (
                SELECT tc.challenge_id 
                FROM temporal_challenges tc 
                WHERE tc.challenge->>'challenge_id' = cp.challenge_id::text
                AND tc.end_at IS NULL
                LIMIT 1
            );
            
            -- Create index on challenge_id for temporal_challenges first
            CREATE INDEX IF NOT EXISTS idx_temporal_challenges_challenge_id_unique ON temporal_challenges(challenge_id) WHERE end_at IS NULL;
            
            -- Note: We cannot create a proper foreign key constraint because challenge_id in temporal_challenges 
            -- is not unique (multiple versions can exist). For now, we'll rely on application-level constraints.
        END IF;
    END IF;
END $$;

-- Update audit_log similarly
DO $$
BEGIN
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'audit_log') THEN
        IF NOT EXISTS (SELECT FROM information_schema.columns 
                      WHERE table_name = 'audit_log' 
                      AND column_name = 'temporal_challenge_id') THEN
            ALTER TABLE audit_log ADD COLUMN temporal_challenge_id INTEGER;
            
            -- Populate the new column
            UPDATE audit_log al
            SET temporal_challenge_id = (
                SELECT tc.challenge_id 
                FROM temporal_challenges tc 
                WHERE tc.challenge->>'challenge_id' = al.challenge_id::text
                AND tc.end_at IS NULL
                LIMIT 1
            );
        END IF;
    END IF;
END $$;

-- Create function to get current challenge version
CREATE OR REPLACE FUNCTION get_current_challenge(p_challenge_id INTEGER)
RETURNS temporal_challenges AS $$
BEGIN
    RETURN (
        SELECT *
        FROM temporal_challenges
        WHERE challenge_id = p_challenge_id
        AND end_at IS NULL
        LIMIT 1
    );
END;
$$ LANGUAGE plpgsql;

-- Create function to create new challenge version
CREATE OR REPLACE FUNCTION create_challenge_version(
    p_challenge_id INTEGER,
    p_challenge_name TEXT,
    p_planned_start_time TIMESTAMP WITH TIME ZONE,
    p_challenge JSONB
)
RETURNS temporal_challenges AS $$
DECLARE
    new_version temporal_challenges;
    current_time TIMESTAMP WITH TIME ZONE := NOW();
BEGIN
    -- End the current version
    UPDATE temporal_challenges
    SET end_at = current_time, updated_at = current_time
    WHERE challenge_id = p_challenge_id AND end_at IS NULL;
    
    -- Create new version
    INSERT INTO temporal_challenges (
        challenge_id,
        challenge_name,
        planned_start_time,
        challenge,
        start_at,
        created_at,
        updated_at
    ) VALUES (
        p_challenge_id,
        p_challenge_name,
        p_planned_start_time,
        p_challenge,
        current_time,
        current_time,
        current_time
    ) RETURNING * INTO new_version;
    
    RETURN new_version;
END;
$$ LANGUAGE plpgsql;