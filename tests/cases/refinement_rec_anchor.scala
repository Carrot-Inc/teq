// A recursive refinement compared with a type that is no path: both sides seen through one anchor,
// a skolem of the left side (`TypeComparer.compareRec`, `ensureStableSingleton`, `fixRecs`).
trait P { val a: String; val b: Any }
type R = P { val b: a.type }
class C extends P { val a: String = "anchor"; val b: a.type = a }
def need[X <: R](x: X): String = x.b
@main def run(): Unit =
  summon[C <:< R]
  val r: R = C()
  println(need[C](C()) + " " + r.b)
