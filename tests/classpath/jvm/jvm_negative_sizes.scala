// jars: scala-library
// std: lean scala-library
// A negative size: the collections take it for none, an array made of it is the JVM's
// `NegativeArraySizeException`.
def attempt(what: String)(body: => Any): Unit =
  try println(what + ": " + body)
  catch case e: Throwable => println(what + " threw " + e.getClass.getSimpleName)

@main def run(): Unit =
  val n = -1
  attempt("List.fill")(List.fill(n)(0))
  attempt("Vector.fill")(Vector.fill(n)(0))
  attempt("List.tabulate")(List.tabulate(n)(identity))
  attempt("Vector.tabulate")(Vector.tabulate(n)(identity))
  attempt("Array.fill")(Array.fill(n)(0).length)
  attempt("Array.tabulate")(Array.tabulate(n)(identity).length)
  attempt("Array.ofDim")(Array.ofDim[Int](n).length)
  attempt("new Array")(new Array[Int](n).length)
  attempt("Array.range")(Array.range(3, 1).length)
  attempt("ArrayBuffer.fill")(scala.collection.mutable.ArrayBuffer.fill(n)(0))
  attempt("Seq.fill")(Seq.fill(n)("a"))
  attempt("IArray.fill")(IArray.fill(n)(0).length)
  attempt("Array.copyOf")(Array.copyOf(Array(1, 2), n).length)
  attempt("take")(Array(1, 2, 3).take(n).length)
  attempt("List.fill 0")(List.fill(0)(0))
