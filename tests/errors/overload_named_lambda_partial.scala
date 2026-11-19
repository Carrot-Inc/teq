// expect: overload_named_lambda_partial.scala:14:21: error: Found:    ((Int, Int)) => Int⏎Required: PartialFunction[(Int, Int), B]
// expect: overload_named_lambda_partial.scala:15:21: error: Found:    ((Int, Int)) => (Int, Int)⏎Required: PartialFunction[(Int, Int), (K2, V2)]
// expect: overload_named_lambda_partial.scala:16:22: error: Found:    Int => Int⏎Required: PartialFunction[Int, Int]
// A function literal named to an overloaded method is typed ahead as a function where its
// alternatives agree on its parameter types (dotty's `Applications.pretypeArgs`), and the
// `NamedArg` kept around it is not converted to a partial function as a bare closure is
// (`Typer.adaptToSubType`, `blockEndingInClosure`). scalac shows the alternative's open result
// type as `Any`; the mismatch's required type is pinned up to it.
object O:
  def g(a: Int, b: PartialFunction[Int, Int]): Int = b(a)
  def g(a: String, b: PartialFunction[Int, Int]): Int = b(a.length)
@main def run(): Unit =
  val m = Map(1 -> 2)
  println(m.collect(pf = kv => kv._1 + kv._2))
  println(m.collect(pf = kv => (kv._1, kv._2)))
  println(O.g(a = 1, b = x => x + 1))
