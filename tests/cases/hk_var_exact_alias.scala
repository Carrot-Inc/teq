// A constructor variable against an alias applied to as many arguments as it has parameters (`M[I]` against
// `Id[A]` of `type Id[A] = A`, Airstream's `signal.idWrap.splitSeq(..)`): the alias bounds the variable, as dotty's
// `TypeComparer.compareAppliedTypeParamRef` constrains one of its bounds, and the variable is solved to it; `I` takes
// `A`, so that the conversion applies and the given for `Splittable[Id]` is the `Id` one, not `List`'s.
import scala.language.implicitConversions
type Id[A] = A
trait Obs[+A]
class Sig[+A](val v: A) extends Obs[A]
class SeqOps[M[_], I](o: Obs[M[I]]):
  def splitSeq(f: I => String)(using s: Splittable[M]): String = s.name
trait Splittable[M[_]] { def name: String }
given Splittable[Id] with { def name = "Id" }
given Splittable[List] with { def name = "List" }
implicit def toOps[M[_], I](o: Obs[M[I]]): SeqOps[M, I] = new SeqOps(o)
extension [M[_], A](c: M[A]) def idWrap: M[Id[A]] = c
@main def main(): Unit =
  val s = new Sig(3)
  println(s.idWrap.splitSeq(_.toString))
  println(new Sig(List(1)).splitSeq(_.toString))
