//! Dictation without a terminal, for desktop integrations such as Omarchy.
//!
//! `ostt dictate start` spawns a short-lived background job that records from
//! the configured microphone. `stop` makes it transcribe with the selected
//! model and paste into the focused app; `cancel` discards the recording.
//! There is no always-on service: the job exits once its result is delivered.
//!
//! Runtime files live in `$XDG_RUNTIME_DIR/ostt/dictation/` (mode 0700):
//! `state.json` (the status widgets read), `control.sock` (stop/cancel/status
//! for the running job), `job.lock` and `control.lock`.

pub(crate) mod control;
pub(crate) mod job;
pub(crate) mod run;
pub(crate) mod state;
