// jars: fixtures
// targets: js interp
// Quote patterns of a macro jar compiled by Scala 3.8.4 (TASTy 28.8), where they are pickled as
// `QUOTEPATTERN`s: a pattern without holes, holes ascribed a type, a type variable, a declared
// one with a bound, a call's argument, and type patterns. A pattern read as a wildcard would
// match the scrutinees scalac's does not (`isUnit(1)`, `kind(a)`).
import fix.qp38.Qp38

object Main:
  val a = 1
  val b = 2
  def main(args: Array[String]): Unit =
    println(Qp38.isUnit(()))
    println(Qp38.isUnit(1))
    println(Qp38.kind(a + b))
    println(Qp38.kind(List(1).head))
    println(Qp38.kind(Some(a)))
    println(Qp38.kind(Qp38.plain(7)))
    println(Qp38.kind(a))
    println(Qp38.typeKind[List[Boolean]])
    println(Qp38.typeKind[Int])
    println(Qp38.typeKind[String])
