// teq: --source future
// Paths `CheckRealizable` accepts, under `-source future`, where the fields count too
// (`memberRealizability`): a recursive field is checked once (`checkedFields`, one set through the
// analysis), a type member's bounds are read as seen from the type (`A <: B` instantiated to
// `Int <: Any`), a generic field's type likewise, and a wildcard unimport beside a wildcard import
// binds the rest.
class Node { val next: Node = this; val x: Int = 1 }
trait Box[A, B] { type X >: A <: B; val x: Int }
class Gen[A] { val next: A = null.asInstanceOf[A]; val x: Int = 3 }
class Game { def pov: Int = 1; val n: Int = 2 }
object Test {
  final lazy val node: Node = new Node
  final lazy val b: Box[Int, Any] = new Box[Int, Any] { type X = Int; val x = 2 }
  final lazy val g: Gen[Int] = new Gen[Int]
  def recursive: Int = { import node.*; x }
  def bounded: Int = { import b.*; x }
  def generic: Int = { import g.*; x }
  def unimported(game: Game): Int = { import game.{pov as _, *}; n }
}
object Main:
  def main(args: Array[String]): Unit =
    println(Test.recursive)
    println(Test.bounded)
    println(Test.generic)
    println(Test.unimported(new Game))
