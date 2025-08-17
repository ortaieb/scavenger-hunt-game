-- Migration: Replace challenges and waypoints tables with JSON-based temporal storage
-- GitHub Issue #6: Replace store of challenge with a json record

-- Step 1: Create sequence for challenge_id
CREATE SEQUENCE IF NOT EXISTS challenge_id_seq;

-- Step 2: Create sequence for challenge_version_id  
CREATE SEQUENCE IF NOT EXISTS challenge_version_id_seq;

-- Step 3: Create new temporal challenges table with JSON storage
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

-- Step 4: Create UUID to integer ID mapping table for migration
CREATE TABLE IF NOT EXISTS challenge_id_mapping (
    old_uuid UUID PRIMARY KEY,
    new_integer_id INTEGER NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Step 5: Create indexes and constraints for performance
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_id ON temporal_challenges(challenge_id);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_temporal ON temporal_challenges(challenge_id, start_at, end_at);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_name ON temporal_challenges(challenge_name);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_start_time ON temporal_challenges(planned_start_time);
CREATE INDEX IF NOT EXISTS idx_temporal_challenges_json ON temporal_challenges USING GIN(challenge);
CREATE INDEX IF NOT EXISTS idx_challenge_id_mapping_uuid ON challenge_id_mapping(old_uuid);
CREATE INDEX IF NOT EXISTS idx_challenge_id_mapping_int ON challenge_id_mapping(new_integer_id);

-- Add unique constraint for current challenges (end_at IS NULL)
CREATE UNIQUE INDEX IF NOT EXISTS idx_temporal_challenges_challenge_id_unique 
ON temporal_challenges(challenge_id) WHERE end_at IS NULL;

-- Step 6: Create view for current (active) challenges
CREATE OR REPLACE VIEW current_challenges AS
SELECT *
FROM temporal_challenges
WHERE end_at IS NULL;

-- Step 7: Data migration and table replacement
DO $$
DECLARE
    challenge_rec RECORD;
    waypoints_json JSONB;
    challenge_json JSONB;
    new_challenge_id INTEGER;
    new_version_id INTEGER;
BEGIN
    -- Only proceed if old tables exist
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'challenges') THEN
        RAISE NOTICE 'Starting migration from relational to temporal challenge storage...';
        
        -- Migrate each challenge
        FOR challenge_rec IN 
            SELECT * FROM challenges ORDER BY created_at
        LOOP
            -- Get next challenge_id from sequence
            SELECT nextval('challenge_id_seq') INTO new_challenge_id;
            
            -- Build waypoints JSON from old waypoints table
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
            
            -- Build complete challenge JSON with new integer ID
            challenge_json := jsonb_build_object(
                'challenge_id', new_challenge_id,
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
                    'migrated_from_relational', true,
                    'original_uuid', challenge_rec.challenge_id
                )
            );
            
            -- Insert into new temporal table
            INSERT INTO temporal_challenges (
                challenge_id,
                challenge_name,
                planned_start_time,
                challenge,
                start_at,
                created_at,
                updated_at
            ) VALUES (
                new_challenge_id,
                challenge_rec.challenge_name,
                challenge_rec.planned_start_time,
                challenge_json,
                challenge_rec.created_at,
                challenge_rec.created_at,
                challenge_rec.updated_at
            ) RETURNING challenge_version_id INTO new_version_id;
            
            -- Store UUID to integer mapping
            INSERT INTO challenge_id_mapping (old_uuid, new_integer_id) 
            VALUES (challenge_rec.challenge_id, new_challenge_id);
            
            RAISE NOTICE 'Migrated challenge: % (UUID: % -> ID: %)', 
                challenge_rec.challenge_name, challenge_rec.challenge_id, new_challenge_id;
        END LOOP;
        
        RAISE NOTICE 'Challenge migration completed. Updating foreign key references...';
    END IF;
END $$;

-- Step 8: Update foreign key references to use new integer IDs
DO $$
BEGIN
    -- Update challenge_participants table
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'challenge_participants') THEN
        RAISE NOTICE 'Updating challenge_participants foreign key references...';
        
        -- Drop existing foreign key constraint
        ALTER TABLE challenge_participants DROP CONSTRAINT IF EXISTS challenge_participants_challenge_id_fkey;
        
        -- Add new integer challenge_id column
        ALTER TABLE challenge_participants ADD COLUMN IF NOT EXISTS new_challenge_id INTEGER;
        
        -- Populate new column using mapping table
        UPDATE challenge_participants cp
        SET new_challenge_id = cim.new_integer_id
        FROM challenge_id_mapping cim
        WHERE cp.challenge_id = cim.old_uuid;
        
        -- Remove rows where mapping failed (orphaned records)
        DELETE FROM challenge_participants WHERE new_challenge_id IS NULL;
        
        -- Drop old UUID column and rename new column
        ALTER TABLE challenge_participants DROP COLUMN challenge_id;
        ALTER TABLE challenge_participants RENAME COLUMN new_challenge_id TO challenge_id;
        ALTER TABLE challenge_participants ALTER COLUMN challenge_id SET NOT NULL;
        
        -- Update current_waypoint_id references (these become obsolete in JSON storage)
        -- For now, set them to NULL since waypoints are now embedded in challenge JSON
        UPDATE challenge_participants SET current_waypoint_id = NULL;
        
        -- Add foreign key constraint to temporal_challenges
        -- Note: Since challenge_id is not unique (multiple versions exist), we reference via a check
        -- For now, we'll rely on application logic to ensure referential integrity
        -- Alternative: Reference challenge_version_id, but that changes the application logic significantly
        -- ALTER TABLE challenge_participants 
        -- ADD CONSTRAINT fk_challenge_participants_temporal_challenge 
        -- FOREIGN KEY (challenge_id) REFERENCES temporal_challenges(challenge_id) ON DELETE CASCADE;
        
        RAISE NOTICE 'Updated challenge_participants table references';
    END IF;
    
    -- Update audit_log table
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'audit_log') THEN
        RAISE NOTICE 'Updating audit_log foreign key references...';
        
        -- Drop existing foreign key constraints
        ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS audit_log_challenge_id_fkey;
        ALTER TABLE audit_log DROP CONSTRAINT IF EXISTS audit_log_waypoint_id_fkey;
        
        -- Add new integer challenge_id column
        ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS new_challenge_id INTEGER;
        
        -- Populate new column using mapping table
        UPDATE audit_log al
        SET new_challenge_id = cim.new_integer_id
        FROM challenge_id_mapping cim
        WHERE al.challenge_id = cim.old_uuid;
        
        -- Drop old UUID column and rename new column
        ALTER TABLE audit_log DROP COLUMN challenge_id;
        ALTER TABLE audit_log RENAME COLUMN new_challenge_id TO challenge_id;
        
        -- Set waypoint_id to NULL (waypoints are now embedded in challenge JSON)
        UPDATE audit_log SET waypoint_id = NULL;
        
        -- Add foreign key constraint to temporal_challenges (nullable)
        -- Note: Disabled for same reason as challenge_participants - challenge_id not unique
        -- ALTER TABLE audit_log 
        -- ADD CONSTRAINT fk_audit_log_temporal_challenge 
        -- FOREIGN KEY (challenge_id) REFERENCES temporal_challenges(challenge_id) ON DELETE SET NULL;
        
        RAISE NOTICE 'Updated audit_log table references';
    END IF;
END $$;

-- Step 9: Drop old tables (final step of replacement)
DO $$
BEGIN
    -- Drop old tables if they exist
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'waypoints') THEN
        RAISE NOTICE 'Dropping old waypoints table...';
        DROP TABLE waypoints CASCADE;
        RAISE NOTICE 'Old waypoints table dropped';
    END IF;
    
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'challenges') THEN
        RAISE NOTICE 'Dropping old challenges table...';
        DROP TABLE challenges CASCADE;
        RAISE NOTICE 'Old challenges table dropped';
    END IF;
    
    RAISE NOTICE 'Table replacement completed! All challenge data now stored in temporal_challenges with JSON format.';
END $$;

-- Step 10: Create helper functions for temporal challenge management
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

-- Step 11: Create convenient views
CREATE OR REPLACE VIEW challenge_summary AS
SELECT 
    tc.challenge_id,
    tc.challenge_version_id,
    tc.challenge_name,
    tc.planned_start_time,
    (tc.challenge->>'challenge_type')::text AS challenge_type,
    (tc.challenge->>'duration_minutes')::integer AS duration_minutes,
    (tc.challenge->>'active')::boolean AS active,
    (tc.challenge->>'actual_start_time')::timestamp with time zone AS actual_start_time,
    jsonb_array_length(tc.challenge->'waypoints') AS waypoint_count,
    tc.start_at AS version_start,
    tc.end_at AS version_end,
    tc.created_at
FROM temporal_challenges tc
WHERE tc.end_at IS NULL;

COMMENT ON VIEW challenge_summary IS 'Summary view of current challenge versions with key fields extracted from JSON';