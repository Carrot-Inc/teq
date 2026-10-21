// jars: cats-effect-kernel-jvm cats-kernel cats-core fs2-core-jvm scodec-bits-jvm
// A loaded class's higher-kinded parameter (fs2's `Stream[+F[_], +O]`, cats-effect's `Resource[F[_], +A]`) given a
// method's own parameter and an alias's eta-expansion, read first in a body (here and in B.scala before its macro call
// suspends it) and again where B.scala, suspended by the macro of the run in M.scala, is typed after the merge: the
// parameter's information is read as the same `[_] =>> Any` on both sides of the merge (`hasSameKindAs`,
// `core/TypeApplications.scala` 290-305), as scalac 3.8.4 accepts the program (the gate of 70fa37159: an
// application's API refused with "type argument F does not have the same kind as its bound [_]").
import cats.effect.kernel.Resource
object A:
  def run(): Int =
    def m[F[_]](x: Int): Option[Resource[F, Int]] = None
    def s[F[_]](bytes: Array[Byte], name: String): Option[fs2.Stream[F, Byte]] = None
    m[Option](1).size + s[Option](Array.empty, "a").size
