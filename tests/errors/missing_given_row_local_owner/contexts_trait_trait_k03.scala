trait T03 {
  val List(r03) = List({ def call(using need: Numeric[String]): Int = 1; call }): @unchecked
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_trait_trait_k03.scala:2:78: error: No given instance of type Numeric[String] was found for parameter need of method call in trait T03
