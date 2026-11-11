class Holder[TC[_], S](val name: String, val tc: TC[S])

trait Derivation:
  type Typeclass[T]
  def plain: String
  def poly[S]: List[S]
  def empty(): String
  def fallback[S]: Typeclass[S]
  inline def viaPlain: String = plain + plain.length
  inline def viaPoly[S]: List[S] = poly[S]
  inline def viaEmpty: String = empty()
  protected inline def step[S](n: String): Holder[Typeclass, S] = new Holder[Typeclass, S](n, fallback[S])
  inline def derive[S](n: String): Holder[Typeclass, S] = step[S](n)

trait Show[A]:
  def show: String

class Shown[A](val show: String) extends Show[A]

object Impl extends Derivation:
  type Typeclass[T] = Show[T]
  def plain: String = "impl"
  def poly[S]: List[S] = Nil
  def empty(): String = "parens"
  def fallback[S]: Show[S] = new Shown[S]("fallback")

@main def run(): Unit =
  println(Impl.viaPlain)
  println(Impl.viaPoly[String])
  println(Impl.viaEmpty)
  val held = Impl.derive[String]("held")
  println(held.name + " " + held.tc.show)
