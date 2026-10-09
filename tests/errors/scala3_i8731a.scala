// Adapted from scala3 tests/neg/i8731a.scala (Apache-2.0, see tests/scala3/README.md).
// expect: expected 'do' before the end marker
// expect: expected 'yield' or 'do' before the end marker
object test:
  while
    3 == 3
  end while  // error: `do` expected
  do ()

  for
    a <- Seq()
  end for    // error: `yield` or `do` expected
  do ()