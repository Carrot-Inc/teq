package q23
package object po23 {
  val r23 = s"${ ({ given call(using need: Numeric[String]): Int = 1; call }) }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k23.scala:3:75: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q23.po23
