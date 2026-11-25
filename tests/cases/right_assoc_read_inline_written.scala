// An inline method whose body calls a right-associative extension as a method, `this.+:(r())(n())`: expanded, its
// arguments run as written, the right one first.
def n(): Int = { println("left"); 1 }
def r(): Int = { println("right"); 2 }
class E:
  extension (l: Int)
    def +:(r: Int): Int = { println("body"); l + r }
  inline def go(): Int = this.+:(r())(n())
@main def run(): Unit = println(E().go())
