A real `teq test --verbose --events` run of one munit case over the overlays' fallback to one worker (`TEQ_THREADS=2`
under `ulimit -v 8000000`): its transcript (`transcript.txt`, the home directory in a JDK warning's jar path written as
`/home/user`) and its events (`events.txt`), exit 0. It prints `teq: alpha/test: no memory mapping for the type store's
overlays; one worker types the build`, an informational note of teq's (src/types.rs `MAPPING_REFUSED`), which
tests/lila-test-verb.sh holds the records' judgment to pass (bench/reference/records.scala `Transcript.notes`). The
run's two channels were captured as one; the stand-in prints them apart again (tests/support/teq_channels.py), the
note on stderr, where teq writes it.
