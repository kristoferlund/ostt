# Security Policy

## Supported Versions

Security fixes are released for the latest version only.

## Reporting a Vulnerability

Do not open a public issue for security problems. Instead, report them
privately through GitHub's
[private vulnerability reporting](https://github.com/kristoferlund/ostt/security/advisories/new).

Please include:

- What the issue is and what an attacker could do with it.
- Steps or a proof of concept to reproduce it.
- The OSTT version (`ostt --version`), your platform, and the provider, model or agent involved.

Leave out API keys, recordings and transcripts. You should get a response
within a week. Once a fix is released, the advisory is published and you are
credited, unless you prefer otherwise.

## Scope

OSTT runs with user permissions. It records audio, sends it to the local model
or cloud provider you choose, and can pass the transcript, and an attached
image, to an AI agent or a shell command. Issues worth reporting include:

- A way for text in a transcript or an attached image to make an agent run commands, read or change files, or use its tools. OSTT runs every built-in agent with its tools switched off, in an empty working directory.
- An agent or its child processes left running after a timeout, a cancel, or OSTT exiting.
- API keys readable by other users, written to logs, or sent anywhere but their provider. Keys are stored in `~/.local/share/ostt/credentials` with `0600` permissions.
- Recordings, transcripts or history readable by other users.
- Access to the daemon's socket from another user.
- Command or argument injection through configuration, model ids, file names, or a provider's or agent's output.
- The install script installing a release whose checksum does not match, or writing outside the locations it documents.

Bash processing actions run the commands you configure, with your permissions,
by design. A problem in your own action's command is not an OSTT
vulnerability.
