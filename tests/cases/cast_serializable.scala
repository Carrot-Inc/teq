// A case class, a case object, a tuple, an enum and its cases are `java.io.Serializable` as they
// are `Product`s, the parents scalac's desugaring gives them (`Desugar.classDef`, an enum through
// `scala.reflect.Enum`): to the typer, to a type test and to a cast. A class that only extends
// `Product`, and a plain class, are none.
import java.io.Serializable
case class P(n: Int)
case object O
class Sub(n: Int) extends P(n)
enum E:
  case One
  case Two(n: Int)
class Plain
class OnlyProduct extends Product:
  def productArity = 0
  def productElement(n: Int): Any = throw new IndexOutOfBoundsException(n.toString)
  def canEqual(that: Any) = false
def attempt(label: String)(f: => Any): Unit =
  try { f; println(label + " ok") }
  catch case _: ClassCastException => println(label + " CCE")
def name(s: Serializable): String = s.toString
@main def run(): Unit =
  attempt("case class") { (P(1): Any).asInstanceOf[Serializable]; () }
  attempt("case object") { (O: Any).asInstanceOf[Serializable]; () }
  attempt("subclass") { (new Sub(2): Any).asInstanceOf[Serializable]; () }
  attempt("tuple") { ((1, 2): Any).asInstanceOf[Serializable]; () }
  attempt("empty tuple") { (EmptyTuple: Any).asInstanceOf[Serializable]; () }
  attempt("enum value") { (E.One: Any).asInstanceOf[Serializable]; () }
  attempt("enum case") { (E.Two(3): Any).asInstanceOf[Serializable]; () }
  attempt("plain") { (new Plain: Any).asInstanceOf[Serializable]; () }
  attempt("only product") { (new OnlyProduct: Any).asInstanceOf[Serializable]; () }
  val values: List[Any] = List(P(1), O, (1, 2), E.One, E.Two(3), new Plain, new OnlyProduct)
  println(values.map(_.isInstanceOf[Serializable]))
  println(values.map {
    case _: Serializable => "s"
    case _ => "-"
  }.mkString)
  val s: Serializable = P(4)
  println(name(s) + " " + name(O) + " " + name((5, 6)) + " " + name(E.Two(7)))
  val both: List[Product & Serializable] = List(P(8), O, E.One)
  println(both.map(_.productPrefix))
