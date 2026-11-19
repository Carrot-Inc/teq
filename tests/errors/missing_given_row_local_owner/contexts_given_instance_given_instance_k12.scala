trait G12
given gi12: G12 with {
  val r12 = for (i <- List({ def call(using need: Numeric[String]): Int = 1; call })) yield i
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k12.scala:3:82: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi12
