package dlastuse

import dlast.*

class Mixed extends Base with Layer

@main def main(): Unit =
  locally { import Givens.given; Givens.use }
  println(new Mixed().f(5))
  println(new Mixed().f())
  println(new Child().go({ print("a"); 3 }))
  var n = 3
  inline def local(a: Int = mark("A") + n)(b: Int = mark("B")): Int = a + b
  println(local()())
  n = 5
  println(local()(mark("C")))
