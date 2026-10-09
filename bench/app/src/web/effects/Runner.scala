package meridian.web.effects

import meridian.core.effect.*

type Callback = Task[Unit]
object Callback:
  val empty: Callback = Eff.unit
  def apply(body: => Unit): Callback = Eff.succeed(body)
  def log(text: => String): Callback = Log.infoEff(text)

/** Runs an application effect from a page: the environment is supplied, a failure is logged. */
final class Runner[R](env: Env[R]):
  def runAction(action: Eff[R, Throwable, Any]): Callback =
    Eff.succeed {
      Runtime.run(action, env) match
        case Exit.Failure(cause) => Log.error(cause.prettyPrint)
        case _ => ()
    }
  def runFetchAction[A](action: Eff[R, Throwable, A]): Task[A] = action.provide(env)
  def runNow(action: Eff[R, Throwable, Any]): Unit = runAction(action).runNow()
  def runSyncAction[A](action: Eff[R, Throwable, A]): A = Runtime.run(action, env).getOrThrow

object Runner:
  given [R]: Tag[Runner[R]] = Tag("Runner")

extension [A](task: Task[A])
  def toCallback: Callback = task.unit
  def logged(label: String): Task[A] = task.tap(a => Log.infoEff(s"$label: $a"))
