# Waypoints Check In

A signal from participants showing arrival of the saught waypoint. Arrival to a challenge waypoint means being less than a `radius` away from the target point.
For the cases the message was sent from inside the radius, the participant check in will be logged, state will be updated and the new state will be sent as part of the response. For cases the messages is still _too far_, a message will be sent, offering a hint.


## Check in Request sent from inside the radius

### Input
```
POST /challenges/waypoints/<waypoint-id>/checkin
headers: auth-token: <participant-auth-token>
{
  location: {
    lat: <double>,
    long: <double>
  }
}
```

### Outcome

```
200 OK
{
  challenge-id: <challenge-unique-id>,
  participant-id: <participant-unique-id>,
  timestamp: 1970-01-01T00:00:00.000+0000,
  waypoint-id: <waypoint-id>,
  state: CHACKED_IN
  proof: <description of the waypoint proof>
}
```


## Check in request outside of the tolerant radius

### Assumptions

The participant sent a `check-id` signal with a location values too far (according to the current waypoint radius)

### Input
```
POST /challenges/waypoints/<waypoint-id>/checkin
headers: auth-token: <participant-auth-token>
{
  location: {
    lat: <double>,
    long: <double>
  }
}
```

### Outcome
```
400 Bad Request
{
  message: "Your checkin attempt is too far from the target"
}
```
