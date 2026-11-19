trait G03
given gi03: G03 with {
  val List(r03) = List({ def call(using need: Numeric[String]): Int = 1; call }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k03.scala:3:78: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi03
