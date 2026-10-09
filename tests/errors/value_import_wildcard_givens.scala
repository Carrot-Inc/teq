// expect: 14:29: error: no given instance of type Codec[Int] was found for parameter x
// A wildcard import from a value leaves its givens out, as one from an object does (scalac: E172
// No given instance of type Codec[Int] was found).
trait Codec[A]:
  def name: String

class Defs:
  given Codec[Int] = new Codec[Int]:
    def name = "int"

@main def run(): Unit =
  val d = Defs()
  import d.*
  println(summon[Codec[Int]].name)
