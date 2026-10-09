// A lazy val's value is assigned to `<name>$v` inside the blocks of its initialiser, a top-level
// one's and a local one's: a local of the initialiser named so is named apart from it.
lazy val x: Int =
  val `x$v` = 5
  1 + `x$v`

def local(): Int =
  lazy val y: Int =
    val `y$v` = 6
    2 + `y$v`
  y

@main def main(): Unit =
  println(x)
  println(local())
