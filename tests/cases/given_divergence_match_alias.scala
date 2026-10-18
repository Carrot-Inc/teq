// Recursive givens over match aliases and type-level arithmetic: the size of an applied match alias or operation is
// that of what it reduces to (`Types.TypeSizeAccumulator`'s `tryNormalize`), so `Succ[Count[3 - 1]]` measures as
// `Succ[Succ[Succ[Zero]]]`. The requests shrink and none diverges, as in scalac, where the unreduced alias would
// grow by one application a level.
import scala.compiletime.ops.int.*

sealed trait Nat { def value: Int }
case object Zero extends Nat { def value = 0 }
final case class Succ[N <: Nat](prev: N) extends Nat { def value = prev.value + 1 }
object Nat:
  given zero: Zero.type = Zero
  given succ[N <: Nat](using n: N): Succ[N] = Succ(n)
  type Count[I <: Int] <: Nat = I match
    case 0 => Zero.type
    case _ => Succ[Count[I - 1]]

trait Show[A] { def show: String }
final case class P[A, B](a: A, b: B)
def mk[A](s: String): Show[A] = new Show[A] { def show = s }
type Drop[T] = T match
  case P[a, b] => b
object Instances:
  given peel[A, B](using s: Show[Drop[P[A, B]]]): Show[P[A, B]] = mk("peel(" + s.show + ")")
  given base[A]: Show[A] = mk("base")

@main def run(): Unit =
  println(summon[Nat.Count[2]].value)
  println(summon[Nat.Count[3]].value)
  println(summon[Nat.Count[4]].value)
  import Instances.given
  println(summon[Show[P[Int, P[Int, Int]]]].show)
