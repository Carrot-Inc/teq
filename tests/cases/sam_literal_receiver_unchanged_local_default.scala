// A local class's constructor default, whose getter is no body of the method around it, holds a literal of
// `Job` whose body calls `synchronized` unqualified. The expectation is scalac's output.
trait Job { def go(): String }
class O(val tag: String):
  def local(): String =
    class L(val j: Job = () => synchronized { tag })
    new L().j.go()
@main def run(): Unit = println(new O("o").local())
