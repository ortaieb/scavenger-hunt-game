# Start Challenge

A request to start a challenge received from a user specifying the challenge-id to start.

## Authrised and Permissioned Request

### Input
```
POST /challenges/start
headers: auth-token: <user-auth-token>
{
  challenge-id: challenge-id-0
}
```

### Outcome

A participant-id will be generated for each user approved its participation, even if it is not attending.

```
201 CREATED
{
  challenge-id: challenge-id-0,
  planned-start-time: 1970-01-01T00:00:00.000+0000,
  actual-start-time: 1970-01-01T00:15:00.000+0000,
  duration: 120,
  participants: [
    {
      user-id: user-1,
      participant-id: participant-1,
    },
    {
      user-id: user-2,
      participant-id: participant-2,
    },
    ...
    {
      user-id: user-n,
      participant-id: participant-n,
    }
  ]
}
```


## Unauthorised or Unpermissioned Request

This will cover either valid token with no `challenge.moderator` role, or user who is not the assigned moderator for the requested challenge.
Follow `Unauthorised or Unpermissioned Requests` rules,
