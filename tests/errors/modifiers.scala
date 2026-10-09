// expect: modifier sealed is not allowed for this definition
// expect: modifier abstract is not allowed for this definition
// expect: modifier open is not allowed for this definition
// expect: repeated modifier final
// expect: modifier lazy is not allowed for this definition
// expect: a vararg parameter must come last
// expect: val parameters may not be call-by-name
// expect: enum EmptyEnum needs at least one case
sealed object Fun
abstract object Foo
final final case class Baz()
open object Op
object O:
  sealed def y: Int = 1
  lazy def z: Int = 2
  def foo(x: String*, y: String): Int = 1
case class ByName(x: => Int)
enum EmptyEnum {}

@main def run(): Unit = println(O.y)
