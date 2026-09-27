# Security Policy

## Reporting

Please do not publish exploitable vulnerabilities in a public issue before a fix is available.

Until a dedicated private reporting channel is configured, repository maintainers should enable GitHub private vulnerability reporting or provide an equivalent private contact mechanism.

## Scope

Security-sensitive areas include:

- command execution by collectors;
- parsing analyzer output;
- path handling;
- report/config deserialization;
- CI integration;
- future plugin loading.

Collectors must not execute shell strings assembled from untrusted input. Prefer explicit executable + argument lists.
