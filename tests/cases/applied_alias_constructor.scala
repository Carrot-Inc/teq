import scala.deriving.Mirror

class Erased[K, FT](val name: String)
class OrElse[A, B]

trait Kind[Up, Tup, Mono[_ <: Up]]:
  type of[M <: Mirror, O <: Up] = M {
    type MirroredType = O; type MirroredMonoType = Mono[O]; type MirroredElemTypes <: Tup
  }
  type ProductInstances[F[_ <: Up], T <: Up] = Erased[this.type, F[T]]
  given mkProductInstances[F[_ <: Up], T <: Up](using gen: of[Mirror.Product, T]): ProductInstances[F, T] =
    new Erased[this.type, F[T]]("product of " + gen.toString)

type Id[t] = t

object K0 extends Kind[Any, Tuple, Id]:
  type |:[F[_], G[_]] = [x] =>> OrElse[F[x], G[F[x]]]

trait Eq[A]
case class P1(x: Int)

object Use:
  import K0.|:
  def plain[A](using inst: K0.ProductInstances[Eq, A]): String = inst.name
  def generic[A](using inst: => K0.ProductInstances[Eq |: List, A]): String = inst.name
  def written(using inst: K0.ProductInstances[Eq |: List, P1]): String = inst.name

@main def appliedAliasConstructor(): Unit =
  println(Use.plain[P1])
  println(Use.generic[P1])
  println(Use.written)
