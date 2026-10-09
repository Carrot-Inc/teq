// A local inline method called before its block reaches its definition is checked as it is
// written: the import the block enters between the call and the definition is in scope for its
// body (`k` is `B.k` for both calls). scalac prints the lines of the .expected file.
object A { val k = 1 }
object B { val k = 2 }
import A.k

@main def run(): Unit =
  val first = g()
  import B.k
  inline def g(): Int = k
  println(first)
  println(g())
