# RelaisDesk PAM wrapper

This directory is derived from `rustdesk-org/pam` commit
`7bfd25510202cd269292cbdd7c71f3977a6fd762`.

RelaisDesk replaces the unmaintained `users` dependency with `nix`, reports
lookup and encoding errors instead of panicking, and limits environment updates
to the PAM environment rather than mutating process-global state from a worker
thread. The original MIT and Apache-2.0 license files are retained here.
