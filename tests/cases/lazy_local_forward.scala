//> using platform jvm
// A local def reads a local lazy val before the lazy val's
// definition runs: dotty's transformStats allocates the
// holder at the block's head, so the read computes the
// value (scalac prints 42 twice). JavaScript left out.
object Main:
  def forward(): Int =
    var first = 0
    def f(): Int = x
    first = f()
    lazy val x: Int = 42
    first + x

  def main(args: Array[String]): Unit =
    println(forward())
    println(forward() / 2)
