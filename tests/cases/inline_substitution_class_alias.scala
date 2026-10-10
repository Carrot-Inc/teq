// A match-type alias of a generic class, named in an inline method of the class through `this`
// (shapeless3's `Kind.this.LiftP[F, gen.MirroredElemTypes]`), is seen from the receiver at the
// expansion: the class's parameters the receiver fixes (`K0`'s `Mono` is `Id`) reach the alias's
// right-hand side, so the size of the lifted tuple is a constant (`TypeOps.asSeenFrom`). scalac
// prints the lines of the .expected file.
import scala.compiletime.constValue
import scala.deriving.Mirror
type Id[t] = t
type Lift0[F[_], T] <: Tuple = T match
  case EmptyTuple => EmptyTuple
  case a *: b => F[a] *: Lift0[F, b]
object Sizes:
  inline def size[K, E <: Tuple]: Int = constValue[Tuple.Size[E]]
trait Kind[Mono[_]]:
  type LiftP[F[_], T] <: Tuple = Mono[T] match
    case Tuple => Lift0[F, Mono[T]]
  inline def count[F[_], T](using gen: Mirror.ProductOf[T]): Int = Sizes.size[Kind.this.type, LiftP[F, gen.MirroredElemTypes]]
  inline def lifted[F[_], T <: Tuple]: Int = Sizes.size[Kind.this.type, LiftP[F, T]]
object K0 extends Kind[Id]
case class P(a: Int, b: String)
case class Q(a: Int, b: String, c: Boolean)
@main def run(): Unit =
  println(K0.count[Option, P])
  println(K0.count[List, Q])
  println(K0.lifted[Option, (Int, Int, Int, Int)])
