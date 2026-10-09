// Adapted from scala3 tests/run/i7031.scala (Apache-2.0, see tests/scala3/README.md).
@main def Test = {
  val a = 5
  val x = 1

    + `a` * 6

  assert(x == 1, x)
}

