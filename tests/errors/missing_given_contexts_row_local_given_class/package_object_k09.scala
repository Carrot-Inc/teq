package q09
package object po09 {
  def byName09(x: => Int): Int = x
  val r09 = byName09({ given call(using need: Numeric[String]): Serializable with {}; call.hashCode })
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k09.scala:4:91: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q09.po09
