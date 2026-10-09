// jars: macrotask-executor
//> using platform js
//> using dep org.scala-js::scala-js-macrotask-executor::1.1.1
// The macrotask executor from its Scala.js jar, its body compiled over the std's `js.Dynamic`
// and `scala.concurrent` (under node it takes the `setImmediate` branch), against the global
// execution context, whose tasks are microtasks as Scala.js's: a task on the global context runs
// before one on the macrotask executor queued earlier, and both after the synchronous code.
import scala.concurrent.{ExecutionContext, Future, Promise}
import org.scalajs.macrotaskexecutor.MacrotaskExecutor

object Main:
  def main(args: Array[String]): Unit =
    MacrotaskExecutor.execute(() => println("macrotask 1"))
    ExecutionContext.global.execute(() => println("microtask 1"))
    MacrotaskExecutor.execute(() => println("macrotask 2"))
    ExecutionContext.global.execute(() => println("microtask 2"))
    val p = Promise[Int]()
    Future(20)(using MacrotaskExecutor).map(_ + 1)(using MacrotaskExecutor).foreach { n =>
      println(s"future $n")
      p.success(n * 2)
    }(using MacrotaskExecutor)
    p.future.foreach(n => println(s"promise $n"))(using MacrotaskExecutor.Implicits.global)
    println("sync")
