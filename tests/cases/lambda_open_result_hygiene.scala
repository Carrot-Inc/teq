// What a lambda's open result leaves alone: a dependent result through its parameter, a call of a
// dependent method under an open result, a local class in the body (Namer's parameter avoidance and
// `fixLevels`), an implicit search over a variable the search fixes (`Some[String]`, not ambiguous), a
// singleton result and a numeric widening of the body.
package lambdaopenresulthygiene

final case class Item[T](title: String, value: T, disabled: Boolean = false)
trait Sv { type N; def n: N }
def newS[X](x: X): Sv { type N = X } = new Sv { type N = X; def n = x }
class Box[T](val value: T)
trait S { type A; val a: A }
def box(s: S): Box[s.A] = new Box(s.a)
def call[R](f: S => R, s: S): R = f(s)
def make[T](t: T): Box[T] = new Box(t)
trait Tag[A]:
  def name: String
given Tag[Option[String]] with
  def name = "option"
given Tag[Some[String]] with
  def name = "some"
def inspect[A](xs: List[Item[A]])(using tag: Tag[A]): String = tag.name

@main def main(): Unit =
  val cl = (s: Sv) => newS(s.n)
  val cl2: (s: Sv) => Sv { type N = s.N } = cl
  println(cl2(newS("ok")).n)
  val s = new S { type A = String; val a = "ok" }
  val b: Box[s.A] = box(s)
  println(b.value)
  println(call(x => box(x), s).value)
  val f = (s: String) =>
    class Local(val value: String)
    make(new Local(s))
  println(f("ok").value.value)
  println(inspect(List("x").map(s => Item(s, Some(s)))))
  val one: Int => 1 = _ => Box(1).value
  println(one(0))
  val longs: List[Long] = List(1, 2).map(x => x)
  println(longs.map(_ + 2147483647L))
