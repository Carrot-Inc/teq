val List(r03) = List({ given call(using need: Numeric[String]): Int = 1; call }): @unchecked
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: empty_package_k03.scala:1:78: error: No given instance of type Numeric[String] was found for parameter need of given instance call
