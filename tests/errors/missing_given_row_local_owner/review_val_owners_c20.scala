class Cr20 { def local(using need: Numeric[String]): Int = 1; val result = { def call(using need: Numeric[String]): Int = 1; call } }
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: review_val_owners_c20.scala:1:130: error: No given instance of type Numeric[String] was found for parameter need of method call in class Cr20
