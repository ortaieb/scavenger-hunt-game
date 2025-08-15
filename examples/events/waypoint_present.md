# Present a Waypoint

Calling `/challenges/waypoints/<waypoint-id>/present` will serve to present the requested waypoint or

## Next Waypoint

### Assumption
Current state record on server shows participent as following:
```
{
  participant-id: <participant-id>,
  waypoint-id: 3,
  state: VERIFIED
}
```

### Input
```
POST /challenges/waypoints/4/present
headers: auth-token: <participant-auth-token>
{}
```

### Outcome
```
200 OK
{
  challenge-id: <challenge-unique-id>,
  participant-id: <participant-unique-id>,
  timestamp: 1970-01-01T00:00:00.000+0000,
  waypoint-id: 4,
  state: CHACKED_IN
}
```


## Last Waypoint

### Assumption
Current state record on server shows participent as following:
```
{
  participant-id: <participant-id>,
  waypoint-id: 9,
  state: VERIFIED
}
```

Waypoint #9 is the last waypoint

### Input
```
POST /challenges/waypoints/10/present
headers: auth-token: <participant-auth-token>
{}
```

### Outcome
```
200 OK
{
  "challenge-status": COMPLETED
  "message": "You have completed successfully all waypoint in the challenge, go to finish point"
}
```
