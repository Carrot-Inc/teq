//> using platform jvm
// (JavaScript left out: teq's JavaScript output does not leave the method from a local lazy initialiser's return.)
// A `return` inside a local lazy val's initialiser leaves the enclosing method (dotty: the initialiser becomes a local
// def, whose return NonLocalReturns turns into a NonLocalReturnControl the method catches by its key), also from
// inside a `synchronized` block in the initialiser and from inside a try/finally there; the lazy val is left
// uncomputed, and a later read in another call computes it afresh.
object Main:
  var log = List.empty[String]
  val lock = new Object

  def plain(c: Boolean): Int =
    lazy val x: Int = { if c then return -1; 1 }
    x + 10

  def inSynchronized(c: Boolean): Int =
    lazy val x: Int = lock.synchronized { if c then return -2; 2 }
    x + 20

  def inFinally(c: Boolean): Int =
    lazy val x: Int = try { if c then return -3; 3 } finally log = s"finally $c" :: log
    x + 30

  def main(args: Array[String]): Unit =
    println(s"${plain(true)} ${plain(false)}")
    println(s"${inSynchronized(true)} ${inSynchronized(false)}")
    println(s"${inFinally(true)} ${inFinally(false)}")
    println(log.reverse.mkString(", "))
