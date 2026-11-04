# Security and user data

Do not publish `data`, account databases, access/refresh tokens, passwords, private keys or personal worlds in a repository or release. Inspect logs before attaching them to an issue; remove credentials and personal server details. A local/offline nickname is not a Minecraft license.

No automated telemetry or remote reporting service is configured by this packaging document. Inspect the source for the actual networking behavior: Microsoft/Minecraft, Modrinth, Prism metadata and optional account integrations communicate with their services.

Report security vulnerabilities privately through GitHub's [Report a vulnerability](https://github.com/minidream0/graphite-launcher/security/advisories/new) feature. Public Issues are for ordinary bugs and feature requests. Do not report credentials or exploitable details in a public issue. Do not include real account tokens even in a private report; use redacted examples.
