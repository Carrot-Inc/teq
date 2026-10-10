// expect: 6:11: error: type mismatch: found Array[Int], required Seq[String] | Array[? <: String]
// A spliced sequence is typed and adapted against a sequence or an array of the repeated
// parameter's elements, and a mismatch is reported against both, as scalac reports it.
def g(ys: String*): Int = 0
val a = Array(1)
val n = g(a*)
