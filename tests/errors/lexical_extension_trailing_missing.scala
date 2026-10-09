// expect: 13:64: error: no given instance of type Need was found
// expect: 1 error found
// A using clause after the method's explicit parameters is the selected extension's, resolved
// after its arguments: its missing given is reported, not a reason to take the companion's
// extension (scalac: "No given instance of type Need was found for parameter x$3 of method
// pick").
trait Need
class R
object R:
  extension (r: R) def pick(x: Int): String = "companion"
object Main:
  extension (r: R) def pick(x: Int)(using Need): String = "lexical"
  def main(args: Array[String]): Unit = println((new R).pick(1))
