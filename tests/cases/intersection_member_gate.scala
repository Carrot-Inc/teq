// Two members of one name and matching signatures on an intersection's parts, merged as dotty's
// `Denotations.mergeSingleDenot` merges them over their whole information: the one whose
// information overrides the other's, else one member of the joint information (`infoMeet`: matching
// parameters, the results met, a name the two disagree on synthetic; a value and a parameterless
// method their types met), else both kept as an overload the application resolves (a value against
// a method of an empty clause, `r.f()` the method's call and `r.f` the value's; a using clause
// against an explicit one; `S`'s `get` beside `R`'s seen from the whole receiver), the shapes no
// class can be both of typed alone. Two type parameters of different bounds meet under the bounds
// met. The same symbol through two instances stays one member (`List[Int] | List[String]`), where
// two declarations of those parameters would be ambiguous.
trait A:
  def f(x: String): Product
  def g(x: Int)(y: Int): Any
trait B:
  def f(x: String): Serializable
  def g(x: Int)(y: Int): Option[Int]
class AB extends A with B:
  def f(x: String): Product & Serializable = Some(x)
  def g(x: Int)(y: Int): Option[Int] = Some(x + y)

trait V:
  val f: String
trait D:
  def f(): String

trait W:
  def f: CharSequence

trait L[K]:
  def f(x: K): Any
class LS extends L[List[Any]]:
  def f(x: List[Any]): Any = x.length

trait R[K <: String]:
  def get(k: K): Any
trait S[K <: String] extends R[K]:
  override def get(k: K): String
class SS extends S[String]:
  def get(k: String): String = "got " + k

trait SA:
  def p[T <: Serializable](x: List[T]): Any
trait SB:
  def p[U <: Product](x: List[U]): String
trait CA:
  def h(x: Int)(using y: String): Product
trait CB:
  def h(x: Int)(y: String): Serializable

def met(r: A & B): Product & Serializable = r.f("k")
def named(r: A & B): Product & Serializable = r.f(x = "n")
def clauses(r: A & B): Option[Int] = r.g(1)(2)
def valueOf(r: V & D): String = r.f
def methodOf(r: V & D): String = r.f()
def valueMet(r: V & W): String = r.f
def valueMetReversed(r: W & V): String = r.f
def nil(r: L[List[Int]] & L[List[String]]): Any = r.f(Nil)
def overriding(r: S["k"] & R["v"]): String = r.get("k")
def overridden(r: S["k"] & R["v"]): Any = r.get("v")
def metBounds(r: SA & SB): List[Serializable & Product] => String = r.p
def explicitClause(r: CA & CB): Serializable = r.h(1)("s")
def usingClause(r: CA & CB)(using String): Product = r.h(1)

@main def main(): Unit =
  val ab = new AB
  println(met(ab))
  println(named(ab))
  println(clauses(ab))
  println(nil((new LS).asInstanceOf[L[List[Int]] & L[List[String]]]))
  val ss = (new SS).asInstanceOf[S["k"] & R["v"]]
  println(overriding(ss))
  println(overridden(ss))
