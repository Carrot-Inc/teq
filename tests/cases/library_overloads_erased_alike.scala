// jars: erasedoverloads-lib
// A library class whose overloads take `Seq[Any]` and `Array[Any]`: its declarations are scalac's, checked when scalac
// compiled them, and its use reports nothing.
@main def run(): Unit =
  val m = new erasedoverloads.PairMaker
  println(m.rawMake(Seq(1, "a")))
  println(m.rawMake(Array[Any](2, "b")))
