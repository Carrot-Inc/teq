package q04
package object po04 {
  val (r04: Int) = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k04.scala:3:74: error: No given instance of type Numeric[String] was found for parameter need of method call in package q04.po04
