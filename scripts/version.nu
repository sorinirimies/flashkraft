#!/usr/bin/env nu
# Prints the current workspace version from Cargo.toml.
# Usage: nu scripts/version.nu

open Cargo.toml | get workspace.package.version | print
