package q22
package object po22 {
  val r22 = if (true) { def call(using need: Numeric[String]): Int = 1; call } else 0
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_package_object_package_object_k22.scala:3:77: error: No given instance of type Numeric[String] was found for parameter need of method call in package q22.po22
