// A given of what a class's self type declares is found through `this`, seen through the self
// type's base type: `given show: Show[A]` of a `Base[A]` under `self: Base[Int] =>`.
trait Show[A]:
  def show(a: A): String

trait Base[A]:
  given show: Show[A]

trait Uses:
  self: Base[Int] =>
  def render(n: Int): String = summon[Show[Int]].show(n)

class Both extends Base[Int], Uses:
  given show: Show[Int] = n => s"<$n>"

@main def main(): Unit =
  println(Both().render(7))
