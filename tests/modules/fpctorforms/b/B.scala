import a2.*
object Run { def main(args: Array[String]): Unit = { println(List(new VaK().n, new DepK().n, new SecK().n, new ByNK().n, new JK().n)); println(U.all); class S extends DepK; println(new S().n) } }
