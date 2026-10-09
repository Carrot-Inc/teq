// The lower bound of a higher-kinded type parameter, `F2[x] >: F[x]`, as fs2's
// `Stream.compile` declares it: an instance for another constructor is no candidate
// (`tests/classpath/js/jar_hk_lower_bound.scala`, `tests/errors/jar_hk_lower_bound.scala`).
package fix.hkb

type HkId[A] = A

trait HkTarget[F[_]]
object HkTarget:
  given HkTarget[List] = new HkTarget[List] {}

sealed trait HkCompiler[F[_], G[_]]:
  def name: String
object HkCompiler extends HkLow:
  implicit def target[F[_]](implicit F: HkTarget[F]): HkCompiler[F, F] = new HkCompiler[F, F] { def name = "target" }
trait HkLow:
  implicit val idInstance: HkCompiler[HkId, HkId] = new HkCompiler[HkId, HkId] { def name = "id" }

final class HkStrm[+F[_], +O]:
  def compile[F2[x] >: F[x], G[_]](implicit c: HkCompiler[F2, G]): String = c.name
