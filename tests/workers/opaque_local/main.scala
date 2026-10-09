// A local class with an opaque type: its class id is the worker's own, and the type store's
// mark of it must not size a table by that id.
@main def run(): Unit =
  class Local:
    opaque type T = Int
    def t: T = 1
    def show(x: T): Int = x
  val l = new Local
  println(l.show(l.t))
