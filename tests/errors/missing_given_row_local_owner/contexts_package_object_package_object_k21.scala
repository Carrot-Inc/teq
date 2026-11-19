package q21
package object po21 {
  val r21 = { given Int = 1; { def call(using need: Numeric[String]): Int = 1; call } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_package_object_package_object_k21.scala:3:84: error: No given instance of type Numeric[String] was found for parameter need of method call in package q21.po21
