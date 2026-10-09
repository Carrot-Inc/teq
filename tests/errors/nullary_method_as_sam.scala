// expect: 18:21: error: method task must be called with () argument
// expect: 19:8: error: method task must be called with () argument
// expect: 20:14: error: method task must be called with () argument
// expect: 21:10: error: method task must be called with () argument
// expect: 22:8: error: method work in object O must be called with () argument
// A method declared with `()` where a SAM is expected (`Runnable`, a program's trait) is no
// function value: dotty eta-expands a method of no parameters for a function type alone
// (`Typer.adaptNoArgs`, arity -1), and a Scala 3 method is not auto-applied either, so each is
// scalac's E100; against `() => Unit` it is expanded, a lambda calling it passes.
def task(): Unit = println("task")
def exec(r: Runnable): Unit = r.run()
trait Job:
  def go(): Unit
def runJob(j: Job): Unit = j.go()
object O:
  def work(): Unit = println("work")
@main def run(): Unit =
  val r: Runnable = task
  exec(task)
  new Thread(task)
  runJob(task)
  exec(O.work)
  val f: () => Unit = task
  f()
  exec(() => task())
