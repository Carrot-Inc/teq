// A case class with a type parameter inside an anonymous class, as cats'
// `catsSddDeferForFunction0` defines its `Deferred` (`tests/classpath/js/jar_local_case_class.scala`):
// the anonymous class's body holds the case class, its companion, and calls of both.
package fix.shapes

trait Deferring[F[_]]:
  def defer[A](fa: => F[A]): F[A]

object LocalDefer:
  implicit val deferFunction0: Deferring[Function0] =
    new Deferring[Function0] {
      case class Deferred[A](fa: () => Function0[A]) extends Function0[A] {
        private lazy val resolved: Function0[A] = {
          @annotation.tailrec
          def loop(f: () => Function0[A]): Function0[A] =
            f() match {
              case Deferred(f) => loop(f)
              case next        => next
            }
          loop(fa)
        }
        def apply(): A = resolved()
      }
      def defer[A](fa: => Function0[A]): Function0[A] = {
        lazy val cachedFa = fa
        Deferred(() => cachedFa)
      }
    }
