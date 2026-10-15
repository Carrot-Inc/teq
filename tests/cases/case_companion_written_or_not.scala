// A case class has a companion object whether or not the source writes one (`Desugar.classDef`'s
// `companionDefs`): its name is that object as a value, which prints the class's name.
case class P(x: Int, y: String)
case class G[A](a: A)
object Holder:
  case class Inner(n: Int)
@main def run(): Unit =
  val c: Any = P
  println(c)
  println(G)
  println(Holder.Inner)
  val f: (Int, String) => P = P.apply
  println(f(1, "z"))
  println(P(2, "w"))
  P(3, "v") match { case P(a, b) => println(s"$a $b") }
