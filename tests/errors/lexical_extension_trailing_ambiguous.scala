// expect: 15:64: error: ambiguous given instances for Need: n1, n2
// expect: 1 error found
// A using clause after the method's explicit parameters is the selected extension's, resolved
// after its arguments: its ambiguous given is reported, not a reason to take the companion's
// extension (scalac: "Ambiguous given instances: both given instance n1 in object Main and
// given instance n2 in object Main match type Need").
trait Need
class R
object R:
  extension (r: R) def pick(x: Int): String = "companion"
object Main:
  given n1: Need = new Need {}
  given n2: Need = new Need {}
  extension (r: R) def pick(x: Int)(using Need): String = "lexical"
  def main(args: Array[String]): Unit = println((new R).pick(1))
