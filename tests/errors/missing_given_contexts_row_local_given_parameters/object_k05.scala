object O05 {
  var r05 = { given call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: object_k05.scala:2:69: error: No given instance of type Numeric[String] was found for parameter need of given instance call in object O05
