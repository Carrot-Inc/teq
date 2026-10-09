object O
trait TC[A]
given TC[O.type] = null

def pick[A <: Singleton](using TC[A]): Int = 1

@main def probe(): Unit = {
  val warm = summon[TC[O.type]]
  println(pick)
}
