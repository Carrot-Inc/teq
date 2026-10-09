// Same-named extension methods called through their owner (`M.app(1)(xs)`, as a library body
// calls `TypeReprMethods.appliedTo(t)(targs)`): the alternative is chosen by the argument
// list that follows the receiver. And `Right(a, b)` on a class with one tuple-typed field:
// the patterns match the tuple's elements.
object M:
  extension (self: Int)
    def app(t: Int): Int = self + t
    def app(ts: List[Int]): Int = self + ts.sum
object Main:
  def main(args: Array[String]): Unit =
    println(M.app(1)(List(2, 3)))
    println(M.app(1)(4))
    val r: Either[String, (Int, String)] = Right((1, "one"))
    r match
      case Right(n, s) => println(s"$n $s")
      case Left(e) => println(e)
    val o: Option[(String, Int)] = Some(("k", 2))
    o match
      case Some(k, v) => println(k + v)
      case None => println("none")
