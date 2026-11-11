// `narrowBound` (TypeOps.scala 636-684): an argument a parameter's own F-bound names (`lazyRefs`, the `LazyRef` of
// `BS`'s `S <: BS[T, S]`, Checking.scala 386) whose bound has wildcards gets that bound massaged, `BS[BS[?, S]#T, S]`,
// as a GADT bound for the check, inside its own bound and outside it alike, for a method's or a class's parameter, an
// abstract type member (whose GADT bound teq has no state for and reads at the check) and Java's
// `BaseStream[T, S <: BaseStream[T, S]]`. scalac 3.8.4 accepts all of it (pi-scratch/r23b/dotty/probe.log: each check
// fails without the GADT bound and holds with it).
import java.util.stream.BaseStream
trait BS[T, S <: BS[T, S]]:
  def n: Int = 1
trait B3[T, U, S <: B3[T, U, S]]
def g1[S <: BS[?, S]](x: BS[?, S]): Int = x.n
def g2[S <: BS[? <: Int, S]](x: BS[? <: Int, S]): Int = x.n + 1
def g3[S <: B3[?, ?, S]](x: B3[?, ?, S]): Int = 3
def g4[S <: BaseStream[?, S]](x: BaseStream[?, S]): Int = 4
trait Q[S <: BS[?, S]]:
  def x: BS[?, S]
trait H:
  type S <: BS[?, S]
  def x: BS[?, S]
class IS extends BS[Int, IS]
@main def main(): Unit =
  val is = new IS
  println(List(g1(is), g2(is), g3(null), g4(null)).mkString(","))
