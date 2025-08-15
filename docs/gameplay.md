# Gameplay

This is how the game Treasure Hunt will be played:

## Jargon and definitions
**User** - any person with app installed and registered

**Moderator** - a user with the responsibilities to coordinate and prepare a challenge
Challenge organiser - synonym to moderator

**Participant** - unique identity of a challenge, either attached to single user or to participant group

**Participant group** - you can take a challenge as a group, to be part of a group you will have to be an invited user, and attach yourself to a group.
Admin - tbd

**Challenge** - a coordinated attempt organised by a moderator to follow the route in a given timeframe

**Personal Scores** - each user will be scored based on the complexity and achievements of challenges it participated in. Scores will be normalised by the system to provide a good reflection across all users.

**Normalisation function** - used to calculate the scores of the user. TBD, will include number of previous challenges, challenges in the last month, distance, speed, age group etc

**Challenge route** - sequence of planned waypoints each participant should pass through during a challenge.

**Planned Waypoint** - a location the participant should pass through as part of a challenge. Each waypoint should have
- geolocation coordinates (lat/long)
- Instruction how to get to it (game clue)
- List of hints the participants may ask for
- (optional) expected time to get to the point in minutes
- Radius from target
- Proof of attending
    - Subject of the image
    - (optional) time range for the photo taken

## Preparations
Before each round can start the organiser (or moderator) will have to prepare

### Challenge route
The moderator should create a the challenge route. For the initial iteration it will be a structured description of the locations the players
### Challenge period
Each challenge round will require setting a start and end time as well as active flag, this flag will allow moderator to make changes to any aspect of the challenge round.
At any time, if the active flag is set, changes will be communicated to the users
### Participants invitation
Each challenge round will require the moderator to invite one or more users (max participants TBD) to take part in the challenge.
Users will be identified by their email address.
Each user will receive an invitation to an agreed email address. If the email address is not linked to any user, it will invite it to register and download the mobile app.

## Take a Challenge
On the agreed time and location the moderator will ‘start the clock’, that will trigger a push for each of the checked-in participants giving them the first clue.
Each participant will have to analyse the clue and try to get to the waypoint. When the participant believes they close enough they will ‘check in’, if the participant location is sufficiently close to target (in the waypoint radius) the description of the proof will be shown to the client (e.g. sign of ‘The Ale and Pie’ pub).
The participant will then send a picture with the proof and wait for confirmation.
If the proof accepted, the next waypoint clue will be shown. If the proof was rejected, explanation will be provided (image not clear, this is what I saw, etc)

Winner will be the first participant completed all clues and provided proofs

A challenge may have updates on other participants events (where each participant is in the challenge, First to pole, etc) or non at all, allowing each person to complete the challenge at their own time and get scores accordingly.

## Long Term Ideas

### Target audience
Payment model, feedbacks I receive are from two groups: recreational activities (predominantly for kid: birthdays, etc) and sport activity drivers.
For the first group participation should be free while moderators will pay for the right to run a challenge. This conforms with the challenge description as presented above. Moderator actively start the match and all the activities will have to take place in a short period of time.
For more of a navigation challenges, moderator will potentially be a bot and instead of sending a push to everyone, the challenge will be set of longer duration allowing every user to check in to the challenge at its own time and advance on its own.

For both cases, the initial trails will have to be based on a challenges routes created by the operator and avoid offering route creation interfaces

### Monetisation
Following the same two target groups and nature of the different games each will play, one of the option is to describe each type of game differently on the same platform where users can be either random users joining a birrhday party, or a full challengers.

Benefit is the ability to use the members catalogue for promotions in later stage.

Recreational participants will be invited and won’t have to pay for the match, the burden of funding will being with the moderators, who will pay for the right to operate a challenge for certain amount of people for a certain amount of time. The operator should be able to offer variety of difficulties and different length and area at that stage.
For Ranked Navigation challenges, each user will pay for the participation in a challenges routes created (subscribe to or checked in TBD)
There might be an opportunity to offer a ‘league’ with a weekly challenge allowing long term subscription to participate in any challenge offered.
