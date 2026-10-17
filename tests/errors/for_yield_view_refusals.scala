// An existing function value is never wrapped to take a view of its result, as scalac refuses it:
// only a lambda's body, written or a for-comprehension's, is adapted to the expected result.
import scala.language.implicitConversions
final class Task[A](val run: () => A):
  def map[B](f: A => B): Task[B] = Task(() => f(run()))
object Task:
  def succeed[A](a: => A): Task[A] = Task(() => a)
trait ToRaw { def raw: Raw }
final case class Raw(name: String, value: String)
object ToRaw:
  implicit def rawToRaw(r: Raw): ToRaw = new ToRaw { def raw = r }
val f: Long => Raw = n => Raw("a", n.toString)
def value: Long => ToRaw = f
def mapped: Task[ToRaw] = Task.succeed(1L).map(f)
// expect: 13:28: error: type mismatch: found Long => Raw, required Long => ToRaw
// expect: 14:48: error: type mismatch: found Long => Raw, required Long => ToRaw
