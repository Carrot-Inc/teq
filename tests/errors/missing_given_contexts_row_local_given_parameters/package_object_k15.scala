package q15
package object po15 {
  val r15 = { while (false) { { given call(using need: Numeric[String]): Int = 1; call } }; 1 }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k15.scala:3:87: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q15.po15
