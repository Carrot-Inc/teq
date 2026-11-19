trait G05
given gi05: G05 with {
  var r05 = { given call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k05.scala:3:69: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object gi05
