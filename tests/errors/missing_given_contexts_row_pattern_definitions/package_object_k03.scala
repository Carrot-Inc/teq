package q03
package object po03 {
  val List(r03) = List({ def call(using need: Numeric[String]): Int = 1; call }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k03.scala:3:78: error: No given instance of type Numeric[String] was found for parameter need of method call in package q03.po03
