// jars: predef-lib
// A jar's bodies through scala-library's Predef: `copyArrayToImmutableIndexedSeq` gives a
// sequence over a copy of the array (zio-test-sbt's runner takes sbt's task array so), which a
// later write to the array does not change; `==` and `!=` of two tuples (zio-test's
// `TestAnnotation.equals`); a prefix operator applied to its evidence clause (its `unary_!`).
@main def run(): Unit =
  val xs = Array("a", "b", "c")
  val seq = predeflib.Arrays.asSeq(xs)
  xs(0) = "z"
  println(seq)
  println(seq.length)
  val ints = Array(5, 6, 7)
  val indexed = predeflib.Arrays.indexed(ints)
  ints(2) = 0
  println(indexed(2) + " " + indexed.length)
  println(s"${predeflib.Pairs.same("a", 1, "a", 1)} ${predeflib.Pairs.same("a", 1, "b", 1)}")
  println(s"${predeflib.Pairs.differ("a", 1, "a", 2)} ${predeflib.Pairs.differ("a", 1, "a", 1)}")
  println(predeflib.Traces.negated(predeflib.Trace(true)))
