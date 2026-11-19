trait G09
given gi09: G09 with {
  def byName09(x: => Int): Int = x
  val r09 = byName09({ def call(using need: Numeric[String]): Int = 1; call })
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k09.scala:4:76: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi09
