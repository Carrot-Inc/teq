sealed object Fun
abstract object Foo
final final case class Baz()
open object Op
object O:
  sealed def y: Int = 1
  lazy def z: Int = 2
  private private val q = 3
@main def run(): Unit = println(O.y)
