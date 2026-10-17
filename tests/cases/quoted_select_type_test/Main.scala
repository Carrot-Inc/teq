import cats.syntax.eq.*
final class Holder(val number: Int):
  def check: String = Inspect.inspect(this.number)
  def ===(other: Int): Boolean = number == other
object Counter:
  val number = 4
  def own: String = Inspect.inspect(this.number)
@main def main(): Unit =
  val segments = List("a", "b")
  println(Inspect.inspect(segments.size === 2))
  println(Inspect.inspect(new Holder(1) === 2))
  println(Holder(3).check)
  println(Counter.own)
  println(Inspect.inspect(segments.forall(_.matches("[a-z]+"))))
