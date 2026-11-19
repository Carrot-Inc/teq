val r11: PartialFunction[Int, Int] = { case i => { given call(using need: Numeric[String]): Serializable with {}; call.hashCode } }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k11.scala:1:119: error: No given instance of type Numeric[String] was found for parameter need of given instance call
