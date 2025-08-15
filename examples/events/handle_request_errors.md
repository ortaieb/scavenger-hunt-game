# Handling Errors


## Unauthorised or Unpermissioned Requests - General Errors

For missing, corrupted or wrong data, use the following to response:

| Case                    |  Status Code  | Error Message                        |
|-------------------------|---------------|--------------------------------------|
| no auth-token header    | 401           | request did not include token        |
| corrupted or user token | 401           | request carries the wrong token      |
| non inflight challenge  | 403           | cannot log location out of challenge |


## Waypoint Validation

### Wrong State
**Outcome Format**: When action does not meet the expected state on server:
- `.../present` when state is not `VERIFIED`
- `.../checkin` when state is not `PRESENTED`
- `.../proof` when state is not `CHECHKED_IN`

**Outcome Format**:
```
409 Conflict
{
  message: "Unexpected Checkin"
}
```

### Wrong Waypoint-id
**Outcome Format**: When request arrive for unexpected waypoint-id
- request `.../(n+1)/present` arrive but participant is not in waypoint (n)
- request `.../(n)/checkin` arrive but participant is not in waypoint (n)
- request `.../(n)/proof` arrive but participant is not in waypoint (n)

**Outcome Format**:
```
409 Conflict
{
  message: "Wrong waypoint"
}
```

### Client Side handling
This outcome (409) will require the participant app to request for the latest state and build screen from the correct state.
