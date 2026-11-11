// `@specialized` arguments that name a value class's companion and `AnyRef`, which teq reads and otherwise ignores.
class Z[@specialized A, @specialized(AnyRef) B](var a: A, var b: B):
  override def toString = "" + ((a, b))

trait Prop[@specialized(Boolean) +T]:
  def value: T

class PropImpl[+T] extends Prop[T]:
  def value: T = scala.sys.error("")

@main def run(): Unit =
  println(new Z("a", 2))
  println(new Z(2, "a"))
  try new PropImpl[Boolean]().value
  catch case e: RuntimeException => println("runtime exception")
