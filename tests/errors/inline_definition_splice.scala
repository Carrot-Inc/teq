// A macro's splice holds an `Expr` of the declared result type at the definition, as scalac 3.8.4
// checks it: `Expr[String]` where an `Int` is declared is an error, called or not, and so is an
// `Expr[Int]` where a `Long` is (no widening); where `Unit` is declared any `Expr` is accepted
// (`tests/cases/inline_definition_splice`).
// expect: type mismatch: found Expr[String], required Expr[Int]
// expect: type mismatch: found Expr[Int], required Expr[Long]
import scala.quoted.*
inline def bad: Int = ${ impl }
def impl(using Quotes): Expr[String] = Expr("oops")
inline def wide: Long = ${ implInt }
def implInt(using Quotes): Expr[Int] = '{ 1 }
@main def run(): Unit = println(1)
