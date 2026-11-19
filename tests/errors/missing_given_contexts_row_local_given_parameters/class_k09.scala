class C09() {
  def byName09(x: => Int): Int = x
  val r09 = byName09({ given call(using need: Numeric[String]): Int = 1; call })
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k09.scala:3:78: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C09
