# Security Policy

## Supported versions

Clipt has no published releases yet. Once it does, only the latest version will receive fixes.

## Reporting a vulnerability

There is no private reporting channel yet — the project is not hosted anywhere public. When it is published, use the hosting platform's private vulnerability reporting (for example, GitHub's "Report a vulnerability" button under the repository's Security tab). Please do not open a public issue for security problems.

Include:

- what the issue is and how to reproduce it
- the impact you believe it has
- the version or commit you tested

You will get an acknowledgment when the report is seen; fixes and disclosure timing are handled case by case.

## Scope

Clipt is a local-only Windows desktop app. It captures clipboard contents — which may include sensitive data such as passwords — and stores them unencrypted under `%APPDATA%\Clipt\` while it runs. By design:

- it makes no network requests, and the UI loads only bundled content
- the database, cached images, and settings are readable by anyone who can read your user profile — local, unencrypted storage is a known property, not a vulnerability
- clipboard data never leaves the machine unless you copy it out yourself

In scope: unintended disclosure of captured data (for example, contents written somewhere unexpected), path or privilege escapes around the media store, and IPC/capability issues (for example, a web page able to drive the app's commands).

Out of scope: local attackers with access to your Windows account, and physical access to the machine — the clipboard and the app data directory are outside Clipt's threat model.
