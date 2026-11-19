package q17
package object po17 {
  val r17 = try 1 catch { case _: Throwable => { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k17.scala:3:117: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q17.po17
