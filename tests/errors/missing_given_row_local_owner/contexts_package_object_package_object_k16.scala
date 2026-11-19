package q16
package object po16 {
  val r16 = try { def call(using need: Numeric[String]): Int = 1; call } catch { case _: Throwable => 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_package_object_package_object_k16.scala:3:71: error: No given instance of type Numeric[String] was found for parameter need of method call in package q16.po16
