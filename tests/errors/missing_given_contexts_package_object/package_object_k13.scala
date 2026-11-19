package p13
package object po13 {
  val r13 = for (i <- List(1) if { def call(using need: Numeric[String]): Int = 1; call } > 0) yield i
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k13.scala:3:88: error: No given instance of type Numeric[String] was found for parameter need of method call
