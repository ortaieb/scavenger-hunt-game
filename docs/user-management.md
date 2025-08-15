# Scavenger Hunt Game - User Mgmt

## Users
Every person, player, orgeniser or administrator taking part in the game will have to have a registered identity in the game.
This will guaranty activities could be tracked and audited.
Users will register with unique identity in their posetion (email address). This information is for login purpose only. In the
game the users will be identifyed by user-handle to be used to present participant lists, leaderboards etc.

## Supported Roles

The preferred method of checking if user is permitted to make activity will be with roles. The enumerated list will allow store
permissions for several aspects, avoid use of `special admin` accounts causing duplication.

These are the roles to be supported:
- **game.admin** super user with the ability to override activities
- **challenge.manager** permissions to create new challenge, set time related, invite users
- **challenge.moderator** permissions to start/end a challenge
- **challenge.participant** user with permission to participate in a challenge
- **challenge.invitee** user with open invitation to a challenge (this distinction will help for paying customers)
- **user.verified** user passed verification and allowed to act in the app

## Sensitive Data

As the game driven from activies in reallife, approval from users to track their data while playing and GDPR compliance will have top priority.

- Each client will give permission to provide access to its location and other details to be used during a challenge
- Competitive participants will have to agree to long term store of their data (even if separated from their identity)
- Challenge entries will be linked onlu by a reference link between the user and the challenge entity
- Challenges data will be scrubbed as soon as possible after the challenge with no way to track the users back from the data.

## Runtime

To achieve secured connection each communication should start with a login call return with an _auth-token_. The auth token will include the user details (name, id).
When a `challenge` begins, each of the users will recieve message, based on thier acceptance to be invited to the challenge. By confirming you start the game,
a secondary auth will be sent with the user-id and the challenge-id, to generate an authentication for the duration of the challenge. The sencond authentication
will include the following:
  - user-id
  - participant-id (unique identifier for the challenge)
  - challenge-id
  - challenge nickname
  - set of roles of the user
  - expiry (end-of-challenge + 1hour)

The user updates will not include any cleartext to disclose either of the above details

Upon receiving of the request, and depending on the endpoint, auth-token will be validated, data will be extracted and used according to the needs of the endpoint.
