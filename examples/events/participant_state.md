# Participant Reports

These are the methods a participant can receive information about its state and history.

## Full

### Input
```
GET /challenges/participant/full
headers
  - auth-token: <participant-auth-token>
```

### Outcome
```json
{
  "participant-id": <participant-id>,
  "challenge": {
    "challenge-id": <challenge-id>,
    "actual-start-time": <timestamp-challenge-actual-start>,
    "waypoint-num": 7
    "waypoints": [
      {
        "waypoint-id": 1
        "presented-time": <timestamp-started-waypoint1>,
        "verified-time": <timestamp-verified-waypoint1>,
        "state": "VERIFIED"
      },
      {
        "waypoint-id": 2
        "presented-time": <timestamp-started-waypoint2>,
        "verified-time": <timestamp-verified-waypoint2>,
        "state": "VERIFIED"
      },
      {
        "waypoint-id": 3
        "presented-time": <timestamp-started-waypoint3>,
        "state": "PRESENTED"
      }
    ]
  }
}
```


## Summary

### Input
```
GET /challenges/participant/summary
headers
  - auth-token: <participant-auth-token>
```

### Outcome
```json
{
  "participant-id": <participant-id>,
  "challenge-id": <challenge-id>,
  "actual-start-time": <timestamp-challenge-actual-start>,
  "waypoint-id": 3,
  "presented-time": <timestamp-started-waypoint1>,
  "state": "PRESENTED"
}
```
