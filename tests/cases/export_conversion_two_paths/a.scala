package sl
import scala.language.implicitConversions
object syntax:
  implicit def optId[A](a: A): OptOps[A] = OptOps(a)
  final class OptOps[A](a: A):
    def some: Option[A] = Some(a)
