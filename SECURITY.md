# Security Policy

susut decodes image files that may come from untrusted sources, using native
code (mozjpeg, libwebp, rav1e and others). A malformed file could trigger a
crash, excessive memory or CPU use, or a memory-safety bug in one of those libraries.

## Supported versions

Only the latest released version receives security fixes.

## Reporting a vulnerability

Please **do not** open a public issue.

- Use GitHub's private reporting: the **Security** tab of
  [thetasdev/susut](https://github.com/thetasdev/susut/security/advisories/new) → *Report a vulnerability*.
- Or email **ikrom@thetas.dev** with the subject `susut security`.

Include the version (`susut --version`), your OS, the command you ran, and a
minimal file or description that reproduces the problem. Please do not send
images that contain real personal data.

You can expect an acknowledgement within 7 days. Fixes and disclosure are
coordinated with you, and reporters are credited in the changelog unless they prefer otherwise.

## Scope

In scope: crashes, hangs, out-of-bounds or memory errors, path traversal or
unintended file overwrite, and metadata that should have been removed but is kept.

Out of scope: vulnerabilities in a dependency that are already public and fixed
upstream (a dependency bump is welcome as a normal pull request), and
decompression-bomb style inputs that only use a lot of memory on very large images.

## Good practice when processing untrusted files

- Run susut as an unprivileged user, ideally in a container or sandbox.
- Write to a separate output directory (`-o`) instead of next to the originals.
- Set memory and time limits (`ulimit`, container limits) for batch jobs.
