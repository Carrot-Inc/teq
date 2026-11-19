// A trait's final val of no written type that overrides a member of a wider type takes its right-hand side's type, a
// constant; the class that mixes the trait in bridges the overridden member's erased signature to the constant's
// getter, and an object's mirror class forwards it: read through the trait, through the member it overrides and
// through two levels, from an anonymous class, a named class and an object.
trait Base0 { def x: Any; def s: Any }
trait Base extends Base0 { def x: Matchable; def s: CharSequence }
trait C extends Base {
  override final val x = 1
  override final val s = "hello"
}
class D extends C
object E extends C
@main def main(): Unit =
  val anon = new C {}
  for o <- List(anon, new D, E) do
    val b: Base = o
    val b0: Base0 = o
    println(s"${o.x} ${b.x} ${b0.x} ${o.s} ${b.s.length} ${b0.s}")
  println(E.x + 1)
