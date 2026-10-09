// A structural call on a value of no `Selectable` type goes through the imported conversion
// (`reflectiveSelectable`), as dotty adapts the qualifier to `Selectable` first; and a refinement
// written alone (`{ def m: Int }`) is a type, a refinement of `AnyRef`.
import scala.reflect.Selectable.reflectiveSelectable

class C:
  def m: Int = 7
  def add(a: Int, b: Int): Int = a + b
  val name: String = "c"

class D:
  def m: Int = 1

trait Show[T]:
  def show(t: T): String

def f(x: { def m: Int }): Int = x.m
def g(x: AnyRef { def add(a: Int, b: Int): Int }): Int = x.add(2, 3)
def h(x: {
  val name: String
  def m: Int
}): String = x.name + x.m
def k(x: { def m: Int; def add(a: Int, b: Int): Int }): Int = x.add(x.m, 1)
def bounded[T: {Show}](t: T): String = summon[Show[T]].show(t)
given Show[Int] with
  def show(t: Int): String = "int " + t

@main def run(): Unit =
  println(f(new C))
  println(f(new D))
  println(g(new C))
  println(h(new C))
  println(k(new C))
  println(bounded(3))
  val xs: List[{ def m: Int }] = List(new C, new D)
  println(xs.map(_.m).sum)
