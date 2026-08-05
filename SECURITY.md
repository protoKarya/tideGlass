<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
| < 0.1   | No        |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

tideGlass follows [wateringHole security standards](https://git.primals.eco/ecoPrimals/wateringHole)
for all ecoPrimals protists. Report vulnerabilities via a **private Forgejo issue**
on the tideGlass repository — not via public GitHub:

https://git.primals.eco/protoKarya/tideGlass/issues/new

Include:

1. **Description** of the vulnerability
2. **Steps to reproduce** (minimal case preferred)
3. **Impact assessment** — what an attacker could achieve
4. **Affected components** — crate, module, or deploy graph

We aim to acknowledge reports within 48 hours and provide a fix or mitigation
within 7 days for critical issues.

## Security Design

tideGlass inherits the ecoPrimals security model:

- **K-Derm topology** — trust boundaries enforced at gate assignment; tideGlass
  runs on westGate with local CAS access (3.21 TB / 452 GB CAS pool, no cross-gate mesh
  for primary data paths)
- **Dark Forest standards** — UDS-only IPC, BTSP-enforced transport, no
  telemetry or phone-home; capability discovery via biomeOS socket scanning
- **Zero unsafe code** — `#![forbid(unsafe_code)]` on all workspace crates
- **Pure Rust dependencies** — `cargo deny` bans `openssl-sys` and `ring` per
  wateringHole ecoBin compliance
- **Provenance by default** — pipeline outputs flow through rhizoCrypt DAG,
  loamSpine certificates, and sweetGrass attribution braids
- **Consent-gated data** — all external datasets fetched via nestGate CAS with
  BLAKE3 content addressing; no ad-hoc HTTP from module code

## Scope

This policy covers the tideGlass workspace (`crates/*` including `tideglass-bin`,
`graphs/`, `validation/`, tooling configs). The UDS socket (`tideglass.sock`) and
Neural API routing path are in scope. For vulnerabilities in upstream primals
(nestGate, barraCuda, biomeOS, rhizoCrypt, loamSpine, sweetGrass), report to
the respective primal's security contact or the private Forgejo issue above.

## License

This security policy is part of tideGlass documentation, licensed under
CC-BY-SA-4.0 as part of the scyBorg triple-copyleft framework.
