trait G18
given gi18: G18 with {
  val r18 = s"${ given call(using need: Numeric[String]): Int = 1; call }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k18.scala:3:72: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi18
