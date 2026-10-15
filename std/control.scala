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

  // scala-library's `Breaks`, which a lean build's class path leaves out with scala-library: `break()`
  // throws the instance's own exception, which its `breakable` around the block catches; another
  // instance's passes through; `tryBreakable(..).catchBreak(..)` gives the block's value or the handler's.
  class Breaks:
    private val breakException = new BreakControl
    def breakable(op: => Unit): Unit =
      try op
      catch
        case ex: BreakControl if ex eq breakException => ()
    sealed trait TryBlock[T]:
      def catchBreak(onBreak: => T): T
    def tryBreakable[T](op: => T): TryBlock[T] = new TryBlock[T]:
      def catchBreak(onBreak: => T): T =
        try op
        catch
          case ex: BreakControl if ex eq breakException => onBreak
    def break(): Nothing = throw breakException
  object Breaks extends Breaks
  private class BreakControl extends ControlThrowable with NoStackTrace

  object NonFatal:
    def apply(t: Throwable): Boolean = t match
      case _: VirtualMachineError | _: InterruptedException | _: LinkageError | _: ControlThrowable => false
      case _ => true
    def unapply(t: Throwable): Option[Throwable] = if apply(t) then Some(t) else None
