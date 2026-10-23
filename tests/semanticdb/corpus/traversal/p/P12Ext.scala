package p12
object Exts:
  extension (x: Int)
    def plus(y: Int): Int = x + y
    def twice: Int = x.plus(x)
  extension [T](xs: List[T])(using o: Ordering[T])
    def mx: T = xs.max
  val a = 1.plus(2)
  val b = plus(1)(2)
  val c = List(1).mx
