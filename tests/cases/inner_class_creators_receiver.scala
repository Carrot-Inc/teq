// A construction of a class nested in a class takes the receiver it names, a prefix or an import,
// before the enclosing `this`, inside the enclosing class too and over several argument lists
// (dotty's `Typer.newExpr`, the outer argument from the class type's prefix).
class P(val n: Int):
  class I(val x: Int)(val y: Int):
    def get = n + x + y
  def make(other: P): Int =
    import other.{I => R}
    new R(3)(0).get

@main def run(): Unit =
  val q = new P(10)
  println(q.I({ println("x"); 1 })({ println("y"); 2 }).get)
  println(new P(10).make(new P(20)))
  locally {
    import q.{I => R}
    println(new R(3)(4).get)
  }
