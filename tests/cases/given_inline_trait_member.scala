// An inline given of a trait read as a member of an object mixing the trait in: the inline
// accessor expands at the selection (dotty's `Inlines.inlineCall` of the member).
trait T:
  inline given x: Int = 42
object O extends T
@main def run(): Unit =
  println(O.x)
  import O.given
  println(summon[Int] + 1)
