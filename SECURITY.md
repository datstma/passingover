# Security policy

## Reporting a vulnerability

Please **don't open a public issue** for security problems. Report them
privately through GitHub instead: on the repository's **Security** tab, choose
**Report a vulnerability**
(<https://github.com/datstma/passingover/security/advisories/new>).

Please include what you found, how to reproduce it and what impact you
expect. You'll get an acknowledgement as soon as the maintainer sees the
report. Passing Over is a spare-time project, so please allow some time for a
fix.

## Scope

Things we especially care about:

- **Location privacy.** The user's position must never leave the device
  except as a request to the configured data sources, rounded to about 1 km.
  Anything that leaks a more precise location, or sends it elsewhere, is a
  security bug.
- Anything that lets a data source response crash or take over the app or CLI.
- Supply-chain issues in our dependencies (we check them with cargo-deny).

## Supported versions

Passing Over has not had a release yet. Fixes go to `main`.
