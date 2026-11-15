// expect: override_refused_once.scala:10:7: error: error overriding method put in trait Ops of type (x: String): String
// expect: 1 error found
// absent: in the JavaScript output
// A method that the override check refuses has that one diagnostic, at its name:
// `put(a: String): Int` overrides both of `Ops[String]`'s alternatives and fits the first alone.
// scalac reports one error here (E164, at the name: "error overriding method put in trait Ops
// of type (x: String): String; method put of type (a: String): Int needs `override` modifier").
trait Ops[A] { def put(x: A): Int; def put(x: String): String = x }
class Two(k: Int) extends Ops[String]:
  def put(a: String): Int = k

object Main:
  def main(a: Array[String]): Unit =
    println(new Two(1).hashCode != 0)
