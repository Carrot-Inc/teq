// Overloads of which one is `protected[C]` or `private[p]` (http4s's deprecated
// `protected[CORSPolicy] def apply` and `private[client] def translate` beside the public ones,
// `tests/classpath/js/jar_qualified_access.scala`): outside `C` and `p` only the public one is an
// alternative.
package fix.shapes

trait QaFunctor[F[_]]
trait QaApplicative[F[_]] extends QaFunctor[F]
object QaApplicative:
  given QaApplicative[Option] = new QaApplicative[Option] {}

class QaPolicy:
  def apply[F[_]: QaApplicative](x: F[Int]): String = "app"
  protected[QaPolicy] def apply[F[_]: QaFunctor](x: F[Int]): String = "fun"

class QaClient:
  private[shapes] def tr[G[_]: QaApplicative](x: G[Int]): String = "old"
  def tr[G[_]](x: G[Int]): String = "new"

// Two conversions to a member `contains_`, one of them `private[shapes]` (cats' binary
// compatibility `FoldableOps.contains_` beside `UnorderedFoldableOps.contains_`): outside the
// package only the other converts.
class QaFoldOps[A](xs: List[A]):
  private[shapes] def contains_(a: A): Boolean = false
class QaUnorderedOps[A](xs: List[A]):
  def contains_(a: A): Boolean = xs.contains(a)
object QaSyntax:
  implicit def qaFoldOps[A](xs: List[A]): QaFoldOps[A] = QaFoldOps(xs)
  implicit def qaUnorderedOps[A](xs: List[A]): QaUnorderedOps[A] = QaUnorderedOps(xs)
