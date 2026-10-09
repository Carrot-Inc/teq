// jars: scala-library
// std: scala-library
// The variance of a jar's opaque type's parameters, from TASTy: scala-library's
// `opaque type IArray[+T]` makes `IArray[(String, Int)]` an `IArray[Any]`, which magnolia's
// `SealedTrait.apply` takes from zio-json's derivation.
def describe(xs: IArray[Any]): String = xs.mkString(",")

@main def run(): Unit =
  val pairs: IArray[(String, Int)] = IArray(("a", 1), ("b", 2))
  println(describe(pairs))
  val words: IArray[String] = IArray("x", "y")
  val anys: IArray[Any] = words
  println(anys.length)
