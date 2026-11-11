#!/bin/sh
# Reads `cargo test` output on stdin, prints the failures and one line of counts, and fails
# when a test failed, when cargo reported an error (a test binary that did not compile or
# link leaves no result line at all), or when no test passed. `--self-test` proves each case.
summary() {
  grep -E "^test result|^error|FAILED|panicked" \
    | awk '/^test result/{p+=$4; f+=$6; i+=$8} /^error/{e++} !/^test result/{print}
        END {print "tests passed", p+0, "failed", f+0, "ignored", i+0, "errors", e+0;
             exit (f > 0 || e > 0 || p == 0)}'
}

if [ "$1" = "--self-test" ]; then
  expect() { # expect STATUS NAME (input on stdin)
    if summary > /dev/null; then got=0; else got=1; fi
    if [ "$got" != "$1" ]; then
      echo "test-summary self-test: '$2' gave $got, wanted $1" >&2
      exit 1
    fi
  }
  printf 'test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured\n' | expect 0 "all pass"
  printf 'test a ... FAILED\ntest result: FAILED. 2 passed; 1 failed; 0 ignored;\n' | expect 1 "a failed test"
  printf 'test result: ok. 3 passed; 0 failed; 0 ignored;\nerror: linking with `cc` failed: exit status: 1\n' | expect 1 "a binary that did not link"
  printf 'error: could not compile `x` (test "integration") due to 1 previous error\n' | expect 1 "nothing ran"
  printf '' | expect 1 "no output at all"
  echo "test-summary: self-test passed"
  exit 0
fi

summary
