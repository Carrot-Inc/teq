// What the macro's runs read of other workers' publications: a top-level function and the
// templates of the calls it makes, a class's body and its check, each typed by whichever worker
// walks this file after the fork, which a file without quoted code is
// (tests/support/fork-one-bundles.txt).
def twice(x: Int): Int = math.max(x, 0) * 2 + "ab".length + x.toString.length

final class Box(val v: Int):
  def get: Int = v + twice(1)
