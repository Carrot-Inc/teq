object TupleInputs:
  def choose(f: ((Int, String)) => Int): String = "unary"
  def choose(f: (Int, String) => Int): String = "binary"
  def one(p: (Int, String)): Int = p._1
  def two(i: Int, s: String): Int = i
@main def runTupleInputs(): Unit =
  println(TupleInputs.choose(TupleInputs.one))
  println(TupleInputs.choose(TupleInputs.two))
  println(TupleInputs.choose((i, s) => i + s.length))
  println(TupleInputs.choose((p: (Int, String)) => p._1))
