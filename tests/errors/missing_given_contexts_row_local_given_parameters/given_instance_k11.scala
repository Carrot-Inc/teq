trait G11
given gi11: G11 with {
  val r11: PartialFunction[Int, Int] = { case i => { given call(using need: Numeric[String]): Int = 1; call } }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k11.scala:3:108: error: No given instance of type Numeric[String] was found for parameter need of given instance call
