package p21
val r21 = { given Int = 1; { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: named_package_k21.scala:2:97: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package p21
