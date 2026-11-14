// A class that implements `java.util.concurrent.Executor`'s `execute` beside a trait's `execute` whose parameter
// is another class of the same simple name answers each by its own, directly and through either type: the lines
// scalac 3.8.4 prints.
package mine { trait Runnable }
trait Jobs { def execute(r: mine.Runnable): String = "mine" }
class Pool extends java.util.concurrent.Executor with Jobs { def execute(command: java.lang.Runnable): Unit = command.run() }
object Main {
  def main(args: Array[String]): Unit = {
    val p = new Pool
    println(p.execute(new mine.Runnable {}))
    p.execute(new java.lang.Runnable { def run(): Unit = println("ran") })
    (p: java.util.concurrent.Executor).execute(() => println("through the executor"))
    println((p: Jobs).execute(new mine.Runnable {}))
  }
}
