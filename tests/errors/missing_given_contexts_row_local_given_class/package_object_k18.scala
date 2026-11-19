package q18
package object po18 {
  val r18 = s"${ given call(using need: Numeric[String]): Serializable with {}; call.hashCode }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k18.scala:3:85: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q18.po18
