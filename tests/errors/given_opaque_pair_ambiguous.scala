// expect: 12:38: error: ambiguous given instances for TC[O.Id]: a, b
// expect: 1 error found
// Where an opaque type is transparent, a given of its underlying type fits it too, and `disambiguate` and
// `healAmbiguous` (Implicits.scala:1406-1447, 1474-1490) rank the two alike without looking through it again: scalac
// 3.8.4 reports "Ambiguous given instances: both given instance a in object O and given instance b in object O match
// type TC[O.Id]".
trait TC[A]
object O:
  opaque type Id = Int
  given a: TC[Int] = new TC[Int] {}
  given b: TC[Id] = new TC[Id] {}
  def chosen: TC[Id] = summon[TC[Id]]
@main def run(): Unit = println(O.chosen)
