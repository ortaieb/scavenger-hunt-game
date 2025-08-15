# Waypoint Proof

Currently the method of prooving your location is by completing a task requires the participant to send an image taken in the location
of the Waypoint (closer than the tolerance radius).

A proof will be jugded by three factors:
- **content**: the image content will be processed by `image-checker` module and will return evaluation to the existance of the proof as provided to the
  participant
- **location**: the image must be taken from inside the radius tolerance from the objective
- **time**: It cannot be an old photo taken, in some cases of competitive challenges that might be even stricktier (future phases).


## Happy path

### Assumptions

The waypoint goal should
- present a `chopped tree trunk`
- 50 meters or less from (-22.3321, 32.0023)
- between 2025-01-01T00:00:00.000Z and 2026-01-01T00:00:00.000Z

The `happypath-example-image.jpeg` is a photo of chopped tree trunk in -22.3321, 32.0021, taken 2025-01-02T00:00:00.000Z

### Input
```
POST /challenges/waypoints/<waypoint-id>/proof
headers
  - auth-token: <participant-auth-token>
  - content-type: form-data
{
  image: <happypath-example-image.jpeg>
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
  state: VERIFIED
}
```


## Missing Criteria

### Assumptions

The waypoint goal should
- present a `chopped tree trunk`
- 50 meters or less from (-22.3321, 32.0023)
- between 2025-01-01T00:00:00.000Z and 2026-01-01T00:00:00.000Z

The `missing-example-image.jpeg` is a photo of two elephants in -21.3321, 32.0021, taken 2025-01-02T00:00:00.000Z

### Input
```
POST /challenges/waypoints/<waypoint-id>/proof
headers
  - auth-token: <participant-auth-token>
  - content-type: form-data
{
  image: <happypath-example-image.jpeg>
}
```

### Outcome
```
400 Bad Request
{
  message: "Failed to provide a proof. [1] Images is not of `chopped tree trunk, [2] Image was taken away from the target"
}
```
