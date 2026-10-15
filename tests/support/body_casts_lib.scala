// What scalac pickles in bodies teq types again (cats-effect's `Dispatcher` and `Resource.start`): a
// cast a GADT match makes of a value of a pattern-bound wildcard's member to the method's type
// (`cancel.$asInstanceOf$[(cancel.type & r.F[Unit]) & F[Unit]]`), and a local case class whose
// constructor's defaults are its synthetic companion's getters.
package bodycasts

import java.util.concurrent.atomic.AtomicReference

private sealed abstract class RegState[+F[_]] extends Product with Serializable
private object RegState:
  final case class Running[F[_]](cancel: F[Unit]) extends RegState[F]
  case object Completed extends RegState[Nothing]

object Bodies:
  def run[F[_]](st: F[Unit]): String =
    def inner[E](fe: F[E], finalizer: Boolean): String = "inner " + fe + " " + finalizer
    val stateR = new AtomicReference[RegState[F]](RegState.Running(st))
    stateR.get() match
      case r: RegState.Running[?] =>
        val cancel = r.cancel
        inner(cancel, true)
      case _ => "other"

  def start: String =
    final case class State(fin: Int = 1, label: String = "none", done: Boolean = false)
    val s = State()
    s.copy(done = true).toString + " " + State(label = "x")
