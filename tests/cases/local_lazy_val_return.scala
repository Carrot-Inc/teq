// A `return` in a local lazy val's initialiser returns from the enclosing def (dotty's `NonLocalReturns`, the key
// captured by the initialiser): the design pass's witness of TASKS 307's JavaScript row, closed on master.
def f(): Int =
  lazy val x: Int = return 7
  x
@main def run(): Unit = println(f())
