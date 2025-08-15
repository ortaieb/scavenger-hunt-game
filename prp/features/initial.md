# Scavenger-Hunt Game

## Feature:

Scavenger-Hunt Game is the server side component of a Scavenger Hunt variation allowing recreational and competitive variation on the game.
For Phase 1
- Focus on rest based polling communication initiated by the clients app (users and participants)
- Connection to a single database schema


## EXAMPLES & DOCUMENTATION:

### Examples

- Check examples under `examples/` direcotry.
- Under `examples/events` will be examples for rest event and the expected response and activities it should trigger

### Documentation:

- Check `docs/` directory (and its sub-directories) for description of the gameplay and the data models
- For information about what required for proof of location, you can check [image-checker](https://github.com/ortaieb/image-checker). This repository
  is the codebase for the service to be used when an analysis of the photo is sent.

## OTHER CONSIDERATIONS:

- The application will be written in Rust
- Application's attributes (e.g. database instance connection details) should be planned as environment variables allowing changes without changing the code itself
- Use a full uri to descirbe location of image. Have the image_base_dir value as env var. Allowing the agent to use local
  storage or cloud managed solution (gcs/s2/blob). For the later, if required, consider additional security details be added (secret/auth key of any kind).
