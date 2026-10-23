// A self alias's type in parentheses is the type, its `=>` the alias's (dotty's `selfType` reads an `infixType`):
// not a function type's parameters, so the members of `Outer` are the alias's and the template's.
class Outer { def x: Int = 1 }
trait Selfish { self: (Outer) =>
  def y: Int = x + self.x
}
object Main:
  def main(args: Array[String]): Unit = println((new Outer with Selfish).y)
