// Adapted from scala3 tests/run/nullAsInstanceOf.scala (Apache-2.0, see tests/scala3/README.md); replaced: `java.util.Objects.equals` with `==`, the types teq lacks (Byte, Short, Float) are left out.
object Test:
  def main(args: Array[String]): Unit =
    val u = null.asInstanceOf[Unit]
    val c = null.asInstanceOf[Char]
    val i = null.asInstanceOf[Int]
    val l = null.asInstanceOf[Long]
    val d = null.asInstanceOf[Double]
    val str = null.asInstanceOf[String]
    val b = null.asInstanceOf[Boolean]

    assertIs((), u)
    assertIs('\u0000', c)
    assertIs(0, i)
    assertIs(0L, l)
    assertIs(0.0, d)
    assertIs(null, str)
    assertIs(false, b)
    println((i, l, b, str))

  def assertIs(expected: Any, actual: Any): Unit =
    assert(expected == actual)
