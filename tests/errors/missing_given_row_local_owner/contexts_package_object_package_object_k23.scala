package q23
package object po23 {
  val r23 = s"${ ({ def call(using need: Numeric[String]): Int = 1; call }) }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_package_object_package_object_k23.scala:3:73: error: No given instance of type Numeric[String] was found for parameter need of method call in package q23.po23
