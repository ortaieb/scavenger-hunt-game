
# Challenge related models

## Challenge

A challenge record is a combination of where you play, when you play and who you play with.
First iteration of the app will not include any managing screens and will require working with a json/yaml representation for the model.

In _Phase 1_ we will support a rest interface to add new, update existing, delete and retrieve challenge as a whole.
- keep rest conventions
- validation will be part of the import process

### Waypoints Sequence

An ordered sequence with a unique ordered index.
Each waypoint will have the following information:
- **waypoint_id: number**: sequence identifier
- **location: GeoLocation**: tuple representing the lat/long of the waypoint
- **radius: number**: error radius from the waypoint (for checkin and proof)
- **clues**:
  - **waypooint_clue: text**: description of the location of the next point to be described to the participant when current waypoint confirmed
  - **hints: text**: 0 - 3 hints helping the participant getto the waypoint
- **waypoint_time: number**: optional field listing number of minutes to get the waypoint, for competitive challenges. default -1 (disabled)
- **proof_of_attending**:
  - **image_subject: text**: description of the content of the image to be sent as a proof

### Time and properties

- **challenge_id: ChallengeId**: link to specific challenge
- **challenge_name: text**: display field showing the title of the challenge
- **challenge_description: text**: description of the challenge, aread to be played, target audiance etc
- **challenge_moderator: UserId**: contact point for questions and coordination, currently single record, in the future might expand to a set
- **start_time: datetime**: represents the beginning of the window for the challenge
- **end_time: datetime**: represents the end of the window for the challenge
- **challenge_type: ChallengeType**:
  - _REC_: recreational, no scoring will be given to the participants. Time range only defines drop-off/pickup times.
  - _COM_: competitive, scoring will be given to waypoints collections in the time range only but challenge can be played outside of time range.
  - _RES_: restricted, competitve for a specific time range

### Challenge Participants

We assign a challenge participant to each registered user.
Each record of participant will have the following details:
- **challenge_user_id: CallengeUserId**: unique identifier for participant of the challenge
- **participant_nickname: text**: prefered nickname to be used in the challenge (default is the user nickname)
- **user_id: UserId**: unique identifier for a registered user (scoring etc)


## ChallengeLog

If the Challenge is the game plan, the ChallengeLog is the series of events happening when the game is played.
It will serve two purposes:
- game runtime, what each participant is doing at a given moment?
- audit and intermediate scoring.
- recover from failure
For that reason, it will be descibed here as a complete Log but will require split and may even include redundencies where the same datapoint will serve the
participants while they play and the audit log.

### Gameplay

While progressing in the challenge, an in-memory copy of the challenge will be kept per user.

A better way to store data should be developed for one of the first phases but from medeling perspective this info should be store in either a compact way in-mem
or on secondary store.

  ```yml
  participant-id: PARTICIPANT-A
  challenge-id: CHALLENGE001
  current-waypoints:
    waypoint-id: 3
    state: CHECKED_IN
    timestamp: 1970-01-01T00:00:00.000+0000
  ```

#### Waypoint states

**state** will support one of the waypoints phase:
- PRESENTED: The participant got the clue to the next
- CHACKED_IN: Participant signaled arrival to waypoint
- PROOF_REQUEST: Participant received instruction for proof of location
- PROCESS_PROOF: Proof of location is being processed by the system
- VERIFIED: Waypoint completed, trigger next waypoint workflow

### Audit Log
Each even will be described by the following:
- time of the event
- reporter (participant/manager/moderator of the challenge)
- type of activity
- parameters used
- outcome
- outcome payload (e.g. calling image-checker will result in the json describing the rejection)

Audit log will be stored on a persistent medium (start with relationan database).
We will start with sync write but it should be changed to an async process in one of the first phases.

#### GeoLocation Log

The app will send every 30 seconds update with the position of the participant, it will be wasteful to keep the information with the event themselve.
Alternatively, it will help calculate distance or even offer visualisation.

Should store:
- participant-id
- timestamp
- GeoLocation

### Disaster Recovery

In the case of system failure, the ChallengeLog will allow revive the state of the inflight challenges. This means the log must be structured
in a method allowing replaying of the sequence to get the state.
