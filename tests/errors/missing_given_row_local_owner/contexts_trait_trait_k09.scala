trait T09 {
  def byName09(x: => Int): Int = x
  val r09 = byName09({ def call(using need: Numeric[String]): Int = 1; call })
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_trait_trait_k09.scala:3:76: error: No given instance of type Numeric[String] was found for parameter need of method call in trait T09
