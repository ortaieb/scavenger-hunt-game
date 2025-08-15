# End Challenge

A request to end a challenge received from a user specifying the challenge-id to start.

## Authrised and Permissioned Request

### Input
```
POST /challenges/end
headers: auth-token: <user-auth-token>
{
  challenge-id: challenge-id-0
}
```

### Outcome

A participant-id will be generated for each user approved its participation, even if it is not attending.

```
200 OK
{
  challenge-id: challenge-id-0,
  planned-start-time: 1970-01-01T00:00:00.000+0000,
  actual-start-time: 1970-01-01T00:15:00.000+0000,
  duration: 120,
  actual-end-time: 1970-01-01T02:15:00.000+0000
}
```


## Unauthorised or Unpermissioned Request

This will cover either valid token with no `challenge.moderator` role, or user who is not the assigned moderator for the requested challenge.
Follow `Unauthorised or Unpermissioned Requests` rules,
