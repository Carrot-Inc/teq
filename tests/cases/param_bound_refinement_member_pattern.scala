// A typed pattern over a scrutinee whose type parameter conforms to the pattern's type through its bound's
// refinement: the binder has the pattern's members, selected and called as before: the bound an alias, reached
// through a second parameter or holding more members; an applied higher-kinded scrutinee and one typed by an
// intersection; the pattern nested, bound by `@`, in a `val`, a `for`, a tuple and a `collect`; a `val` member, a
// member with an argument, the binder in the guard, unused, passed on, used as the parameter's type; the same
// match twice. scalac 3.8.4 prints the same line.
import scala.reflect.Selectable.reflectiveSelectable

class Knot { def f: Int = 1; val n: Int = 2; def add(a: Int): Int = a + 3; def more: Int = 4 }
trait Named
class Tied extends Knot with Named
class Loop[X](x: X) { def f: X = x }
type Has = AnyRef { def f: Int }
type HasToo = Has
def use(r: Has): Int = r.f
def same[A](a: A, b: A): Int = 94

def pat[T <: Has](t: T): Int = t match
  case r: Has => r.f
def patAlias[T <: HasToo](t: T): Int = t match
  case r: (AnyRef { def f: Int }) => r.f
def patApplied[F[X] <: AnyRef { def f: X }](t: F[Int]): Int = t match
  case r: Has => r.f + 10
def patNested[T <: Has](o: Option[T]): Int = o match
  case Some(r: Has) => r.f
  case None => 0
def patAt[T <: Has](t: T): Int = t match
  case r @ (_: Has) => r.f
def patVal[T <: Has](t: T): Int = { val (r: Has) = t: @unchecked; r.f }
def patFor[T <: Has](t: T): Int = (for case r: Has <- List(t) yield r.f).head
def patInter[T <: Has](t: T & Named): Int = t match
  case r: Has => r.f
def patTuple[T <: Has](t: T): Int = (t, 40) match
  case (r: Has, n) => r.f + n
def patCollect[T <: Has](t: T): Int = List(t).collect { case r: Has => r.f + 50 }.head
def patChain[T <: Has, U <: T](u: U): Int = u match
  case r: Has => r.f
def patMore[T <: AnyRef { def f: Int; def more: Int }](t: T): Int = t match
  case r: Has => r.f + 60
def patValMember[T <: AnyRef { val n: Int }](t: T): Int = t match
  case r: (AnyRef { val n: Int }) => r.n
def patArg[T <: AnyRef { def add(a: Int): Int }](t: T): Int = t match
  case r: (AnyRef { def add(a: Int): Int }) => r.add(70)
def patGuard[T <: Has](t: T): Int = t match
  case r: Has if r.f > 0 => r.f + 80
  case _ => 0
def patUnused[T <: Has](t: T): Int = t match
  case r: Has => 90
def patPassed[T <: Has](t: T): Int = t match
  case r: Has => use(r) + 91
def patCast[T <: Has](t: T): Int = if t.isInstanceOf[Has] then t.asInstanceOf[Has].f + 92 else 0
def patAsParam[T <: Has](t: T): Int = t match
  case r: Has => same[T](r, t)
def patTwice[T <: Has](t: T): Int = (t match { case r: Has => r.f }) + (t match { case r: Has => r.f + 94 })

@main def main(): Unit =
  val k = new Knot
  val tied = new Tied
  println(List(pat(k), patAlias(k), patApplied[Loop](new Loop(1)), patNested(Some(k)), patAt(k), patVal(k), patFor(k), patInter[Tied](tied),
    patTuple(k), patCollect(k), patChain[Knot, Knot](k), patMore(k), patValMember(k), patArg(k), patGuard(k), patUnused(k), patPassed(k),
    patCast(k), patAsParam(k), patTwice(k)).mkString(" "))
