package q22
package object po22 {
  val r22 = if (true) { given call(using need: Numeric[String]): Int = 1; call } else 0
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k22.scala:3:79: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q22.po22
