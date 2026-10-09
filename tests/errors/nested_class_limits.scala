// expect: 9:24: error: not supported yet: an instance of Inner, a class nested in a class, made outside that class
// An instance of a class nested in a class is made through a prefix (`new o.Inner(1)`) or
// inside the class; without either the enclosing instance is not known.
class Outer(val n: Int):
  class Inner(val m: Int)

def make(o: Outer): Outer#Inner =
  import o.*
  val i: Outer#Inner = new Inner(1)
  i

@main def run(): Unit = println(make(Outer(1)).m)
