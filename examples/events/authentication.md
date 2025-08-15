# Authentication

We will cover here two different authentication usecase. The first is logging in to the app. The second is generating a participant-token for a challenge.

## User authentication

### Data Store

User details will be stored in a database table. On Phase 1, password will be clear text.

- **USERS** table
  - user_id: numeric id of the record
  - username: an email address the user identify by (unique)
  - password: clear text `Password1` by default
  - nickname
  - creation_date

  Indexed by username

- **USER_ROLES** table
  - user_id
  - role_id

  Index by user_id, unique (user_id,role_id) tuples


## User Login

User login will verify username/password against the USERS table and create a JWT token.

### Input
```
POST /authentication/login
headers
  - content-type: application/json
{
  username: <username>,
  password: <password>
}
```

### Outcome
Is authenticated correctly, the response will be a JWT token with the following details:

  - issuer: scavenger-hunt-game
  - upn: username
  - groups: roles of the user
  - cliam("exp"): 2 hours window

```
201 CREATED
{
  user-auth-token: <token>,
  expires_in: <expiration-window>,
  token_type: "Bearer"
}
```


## User Registration
For Phase1, a POST /autherntication/register with full users details (user-id will be created by the databnase).
```json
{
  "username": <username>,
  "password": <password>,
  "nickname": <nickname>,
  "roles": [
    <set of roles>
  ]
}
```
`/autherntication/register` will require a `game.admin` role of the user calling it. Inside a transaction it will generate the USERS table record and the use the user_id for the roles.


## Participant Token

### Input
```
POST /challenge/authentication
headers:
  - auth-token: <user-auth-token>
  {
    "challange-id": <challenge-id>
  }
```

### Outcome
  1. If user is linked to the challenge specified the response will be with a participant token.
     The toket will include:
     - issuer: scavenger-hunt-challenge
     - upn: participant-id (attached to the user-id)
     - groups: roles of the user
     - claim("clg"): challenge-id
     - claim("usr"): user-id
     - claim("exp"): challenge-window

     ```
     201 CREATED
     {
       user-auth-token: <token>,
       expires_in: <expiration-window>,
       token_type: "Bearer"
     }
     ```
  2. If the user was not invited to the challenge
     ```
     403 Forbiden
     {
       "message": "no participant attached to the challenge for this user"
     }
     ```
