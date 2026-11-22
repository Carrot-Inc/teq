// Member types read from their source at any depth: a static class through the companion, an enum, a record and an
// interface declared in a class.
object Main {
  def main(args: Array[String]): Unit = {
    val k: J.K = () => 6
    println(s"${new J.N().n()} ${new J.N.M().m()} ${J.E.Q.ordinal} ${new J.P(5).v()} ${k.k()} ${J.make().n()}")
  }
}
