// A lambda of a tuple's elements where a partial function of the tuple is expected: its
// parameters are bound from the tuple before its body runs, so the function is defined
// everywhere and a `match` in its body throws where no case applies, as scalac's desugaring and
// `ExpandSAMs` make it. A lambda of the tuple itself whose body is a match is defined where a case is.
object Main:
  def fallback(p: (Int, String)): String = "fallback"

  def main(args: Array[String]): Unit =
    val pairs = List((1, "a"), (2, "b"))
    println(pairs.collect((k, v) => v * k))
    println(pairs.collect((k: Int, v: String) => v + k))
    println(pairs.collect((k: Int, v: Any) => s"$v:$k"))
    println(pairs.collect((k, v) => { val n = v.length; n + k }))

    val untupled: PartialFunction[(Int, String), String] = (k, v) => k match { case 1 => v }
    println(untupled.isDefinedAt((2, "b")))
    println(untupled.isDefinedAt((1, "a")))
    println(untupled((1, "a")))
    try println(untupled.applyOrElse((2, "b"), fallback))
    catch case _: MatchError => println("MatchError")

    val matched: PartialFunction[(Int, String), String] = p => p match { case (1, v) => v }
    println(matched.isDefinedAt((2, "b")))
    println(matched.applyOrElse((2, "b"), fallback))
    println(matched.applyOrElse((1, "a"), fallback))

    val triple: PartialFunction[(Int, String, Boolean), String] = (k, v, b) => if b then v * k else v
    println(triple((2, "c", true)))
    println(triple.lift((2, "c", false)))
