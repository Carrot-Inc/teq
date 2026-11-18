import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) = Mk.empty
object Mk:
  def sink[T]: T => Int = _ => 1
  def empty[T]: List[T] = Nil
