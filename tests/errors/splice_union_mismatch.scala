// expect: 6:11: error: Found:    Array[Int]⏎Required: Seq[String] | Array[? <: String]
// A spliced sequence is typed and adapted against a sequence or an array of the repeated
// parameter's elements, and a mismatch is reported against both, as scalac reports it.
def g(ys: String*): Int = 0
val a = Array(1)
val n = g(a*)
