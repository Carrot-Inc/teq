case class O()
trait TC[A]
given TC[O] = null

def pick[A <: Equals](using TC[A]): Int = 1

@main def probe(): Unit = {
  val warm = summon[TC[O]]
  println(pick)
}
