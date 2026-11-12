//> using platform jvm
// jars: javafix
// With a class path the std's `Future` is to take the JDK class's members
// and its module, whose static member class `Future.State` (JDK 19) is the JDK's, as dotty's ClassfileParser enters
// it (`enterOwnInnerClasses`); `state(): Future.State` is then a nullary method. The std's FutureTask's
// `private var state = 0` is to match it and inherit nothing, as `Namer.inferredResultType`'s `instantiatedResType`
// gives NoType for a val over a MethodType. A val over a Java nullary method keeps its own type the same way
// (`exceptionNow()`, JDK 19). The module is a term an import names (`import java.util.concurrent.Future.State`).
// The javafix jar is there for the class path alone.
import java.util.concurrent.{CancellationException, FutureTask}
import java.util.concurrent.Future.State

class Failed extends FutureTask[Int](() => 0):
  override val exceptionNow = new IllegalStateException("cause")

@main def run(): Unit =
  val task = new FutureTask[Int](() => 42)
  println(task.isDone())
  task.run()
  println(task.get())
  val other = new FutureTask[String](() => "never")
  println(other.cancel(false))
  println(other.isCancelled())
  try other.get() catch case _: CancellationException => println("cancelled")
  val cause: IllegalStateException = Failed().exceptionNow
  println(cause.getMessage)
  val none: java.util.concurrent.Future.State = null
  println(none == null)
  val imported: State = null
  println(imported == null)
