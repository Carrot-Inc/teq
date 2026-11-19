trait G05
given gi05: G05 with {
  var r05 = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_given_instance_given_instance_k05.scala:3:67: error: No given instance of type Numeric[String] was found for parameter need of method call in object gi05
