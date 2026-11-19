package q01
package object po01 {
  val r01 = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_package_object_package_object_k01.scala:3:67: error: No given instance of type Numeric[String] was found for parameter need of method call in package q01.po01
