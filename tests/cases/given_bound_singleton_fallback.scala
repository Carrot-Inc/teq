object O
trait TC[A] { def n: Int }
trait Low { given fb[A]: TC[A] = new TC[A] { def n = 0 } }
object TC extends Low { given o: TC[O.type] = new TC[O.type] { def n = 1 } }

def pick[A <: Singleton](using tc: TC[A]): Int = tc.n

@main def probe(): Unit = println(pick)
