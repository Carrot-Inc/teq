// An intersection's member as dotty's `Denotation.meet` makes it: members merge only where
// their signatures match (`f[T](x: T)` and `f[U <: String](x: U)` erase apart and stay two
// alternatives, the more specific chosen); one symbol through two instances of its owner meets
// its information (`G[Any] & G[String]`'s `get` is a `String`); an overloaded name meets its
// matching alternatives and keeps the others; a selection's prefix is its qualifier as written,
// an ascription's type included.
trait C:
  def f[T](x: T): Any = "wrong"
trait R:
  def f[U <: String](x: U): U
class Both extends C with R:
  override def f[U <: String](x: U): U = x

trait G[A]:
  def get: A
class GS extends G[String]:
  def get: String = "same"

trait Over:
  def g(x: Int): Any = "over-int"
  def g(x: Boolean): Any = "over-bool"
trait Under:
  def g(x: Int): String
  def g(x: String): String
class OverUnder extends Over with Under:
  override def g(x: Int): String = "met-int"
  def g(x: String): String = "under-string"

trait PA:
  def n: Int
trait PB:
  def n: Int
class PAB extends PA with PB:
  def n = 7
class PBA extends PB with PA:
  def n = 8

def poly(x: C & R): String = x.f("ok")
def same(x: G[Any] & G[String]): String = x.get
def overInt(x: Over & Under): String = x.g(1)
def overString(x: Over & Under): String = x.g("s")
def overBool(x: Over & Under): Any = x.g(true)
def ascribed(flag: Boolean, a: PAB, b: PBA): Int = ((if flag then a else b): PA & PB).n
def joined(flag: Boolean, a: PAB, b: PBA): Int = (if flag then a else b).n

@main def main(): Unit =
  println(poly(new Both))
  println(same((new GS).asInstanceOf[G[Any] & G[String]]).length)
  val ou = new OverUnder
  println(overInt(ou).toUpperCase)
  println(overString(ou))
  println(overBool(ou))
  println(ascribed(true, new PAB, new PBA) + ascribed(false, new PAB, new PBA))
  println(joined(false, new PAB, new PBA))
