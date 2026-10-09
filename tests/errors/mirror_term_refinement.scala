// expect: 8:73: error: no given instance of type Product{type MirroredType = P; def foo: Int} was found for parameter x
// expect: 9:73: error: no given instance of type Product{type MirroredType = P; val foo: Int} was found for parameter x
// expect: 2 errors found
// A mirror target with a term member no mirror has is no given.
import scala.deriving.Mirror
case class P(i: Int)
@main def main(): Unit =
  val a = summon[Mirror.Product { type MirroredType = P; def foo: Int }]
  val b = summon[Mirror.Product { type MirroredType = P; val foo: Int }]
