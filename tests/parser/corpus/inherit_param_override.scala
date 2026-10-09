def side(n: Int): Int = { println("init " + n); n }
trait Foo:
  val someField: Int = side(5)
  def twice = someField * 2
class SimpleFoo extends Foo
class Bar(override val someField: Int) extends Foo
class Base:
  val size: Int = side(1)
  println("Base sees " + size)
class Sized(override val size: Int) extends Base
class Quiet:
  val depth: Int = side(2)
class Deep(override val depth: Int) extends Quiet
class Later extends Deep(7):
  override val depth = side(9)
@main def run(): Unit =
  println(SimpleFoo().twice)
  println(Bar(44).twice)
  println(Sized(3).size)
  println(Later().depth)
