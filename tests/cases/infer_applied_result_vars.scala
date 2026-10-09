// A polymorphic method whose result is applied at once (zio's `ZIO.scoped[R]` then its
// `apply`): the variables the arguments leave open are solved as an application's are, so an `R`
// only the result holds contravariantly is `Any`, not `Nothing`.
trait Scope
trait Args
class Box[-R, +A](val a: A):
  def unit: Box[R, Unit] = Box(())
class Scoped[R]:
  def apply[A](x: Box[Scope & R, A]): Box[R, A] = Box(x.a)
def scoped[R]: Scoped[R] = Scoped[R]

object Main:
  def mk: Box[Scope, Int] = Box(1)
  val d: Box[Args, Unit] = scoped(mk).unit
  def main(args: Array[String]): Unit = println(d.a == ())
