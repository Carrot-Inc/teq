package prober18
val resultr18 = { def call(using need: Numeric[String]): Int = 1; call }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: review_val_owners_c18.scala:2:71: error: No given instance of type Numeric[String] was found for parameter need of method call in package prober18
