#!/usr/bin/env nu
# ── FlashKraft · test_fuzz_all.nu ─────────────────────────────────────────────────
# Tests for scripts/ci/fuzz_all.nu — toolchain fallbacks and cargo fuzz arguments.

use std/assert
use runner.nu *
use ../ci/fuzz_all.nu [fuzz_args toolchains FALLBACK_DAYS]

def "test fuzz_all: latest nightly first then dated fallbacks" [] {
    let tcs = toolchains ("2026-11-01T12:00:00+00:00" | into datetime)
    assert equal $tcs [nightly nightly-2026-10-25 nightly-2026-10-18]
    assert equal ($tcs | length) (($FALLBACK_DAYS | length) + 1)
}

def "test fuzz_all: the chosen toolchain with a time limit" [] {
    assert equal (fuzz_args nightly parse 30) [+nightly fuzz run parse -- -max_total_time=30]
    assert equal (fuzz_args nightly-2026-10-25 parse 30) [+nightly-2026-10-25 fuzz run parse -- -max_total_time=30]
}

def "test fuzz_all: a corpus dir gets one subdir per target" [] {
    assert equal (fuzz_args nightly parse 30 /tmp/corpus) [+nightly fuzz run parse /tmp/corpus/parse -- -max_total_time=30]
}

# ── Main ────────────────────────────────────────────────────────────────────

def main [] { run-tests }
