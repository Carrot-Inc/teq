// An array where a collection is expected is wrapped by Predef's conversion: as an argument or a
// value of `IterableOnce[A]`, and as the receiver of an extension on `IterableOnce[A]`, whose `A`
// the wrapped array fixes (scalac: "a-b", "3", "x/y", "6").
extension [A](as: IterableOnce[A])
  def joined: String = as.iterator.mkString("-")
  def slashed: String = as.iterator.mkString("/")

def total(xs: IterableOnce[Int]): Int = xs.iterator.sum

@main def main(): Unit =
  println(Array("a", "b").joined)
  val it: IterableOnce[Int] = Array(1, 2, 3)
  println(it.iterator.size)
  println(Array("x", "y").slashed)
  println(total(Array(1, 2, 3)))
