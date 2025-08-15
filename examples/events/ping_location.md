# Ping Location

While in a challenge the participant's app will send updates of its location evey configured interval of time (game-wide, not a challenge sepcific).

At first it will be a keepalive only message and will only be logged to the dedicated table. In later phases, the location of the participants will
allow moderator communication when participant has lost its way or to visualise the travel.

The message requires a challenge auth token and will use the participant details rather than the actual user.

## Authrised and Permissioned Request

### Input
```
POST /challenges/location-ping
headers: auth-token: <participant-auth-token>
{
  lat: <double>
  long: <double>
}
```

### Outcome
```
200 OK
```

## Unauthorised or Unpermissioned Request

This will cover either valid token with no `challenge.moderator` role, or user who is not the assigned moderator for the requested challenge.
Follow `Unauthorised or Unpermissioned Requests` rules,
