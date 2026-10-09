// The exception classes of package `scala` and the extractors and marker traits of
// `scala.util.control`.
package scala:
  type NoSuchElementException = java.util.NoSuchElementException

  final class MatchError(obj: Any) extends RuntimeException:
    override def getMessage: String = matchErrorMessage(obj)

  @js("($str($0) + \" (of class \" + $classOf($0) + \")\")")
  @jvm("rt $0:L rtcall matchErrorMessage(Ljava/lang/Object;)Ljava/lang/String;")
  def matchErrorMessage(obj: Any): String

  final class NotImplementedError(message: String = "an implementation is missing") extends Error(message)

package scala.util.control:
  abstract class ControlThrowable(message: String = null) extends Throwable(message)

  trait NoStackTrace extends Throwable:
    override def fillInStackTrace(): Throwable = this

  object NonFatal:
    def apply(t: Throwable): Boolean = t match
      case _: VirtualMachineError | _: InterruptedException | _: LinkageError | _: ControlThrowable => false
      case _ => true
    def unapply(t: Throwable): Option[Throwable] = if apply(t) then Some(t) else None
