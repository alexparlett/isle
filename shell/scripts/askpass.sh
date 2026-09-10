#!/bin/sh
# SSH_ASKPASS for the shell: the passphrase sits in a file the caller made, read once and removed.
f="$ISLE_ASKPASS_FILE"
[ -n "$f" ] && [ -f "$f" ] && { cat "$f"; rm -f "$f"; }
