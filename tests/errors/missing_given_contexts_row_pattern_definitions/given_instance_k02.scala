trait G02
given gi02: G02 with {
  val (r02, s02) = ({ def call(using need: Numeric[String]): Int = 1; call }, 2)
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k02.scala:3:75: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi02
