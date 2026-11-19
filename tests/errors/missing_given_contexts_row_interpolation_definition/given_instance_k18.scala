trait G18
given gi18: G18 with {
  val r18 = s"${ def call(using need: Numeric[String]): Int = 1; call }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k18.scala:3:70: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi18
