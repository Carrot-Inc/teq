// A conversion whose result is its own type parameter still gives what scalac's gives: a lower bound (`T >: Plain`)
// or a using argument fixes the parameter, so the selection finds its member; held inside a covariant result
// (`List[T]`) it is `Nothing` and the member is `List`'s; against an expected type the conversion is taken whatever
// the parameter is. Selected on the parameter bounded above alone, the member is refused
// (tests/errors/conversion_own_parameter_from_below.scala).
import scala.language.implicitConversions
class Rich(val s: String) { def shout: String = s }
class Plain(s: String) extends Rich(s)
object Lower { implicit def conv[T >: Plain <: Rich](b: Boolean): T = new Plain("lower").asInstanceOf[T] }
object Upper { implicit def conv[T <: Rich](b: Boolean): T = new Rich("expected").asInstanceOf[T] }
object Listed { implicit def conv[T <: Rich](b: Boolean): List[T] = List(new Rich("listed").asInstanceOf[T]) }
object Fixed { implicit def conv[T <: Rich](b: Boolean)(implicit w: T): T = w }
def lower(): String = { import Lower.conv; true.shout }
def ascribed(): String = { import Upper.conv; val r: Rich = true; r.shout }
def listed(): Int = { import Listed.conv; true.size }
def fixed(): String = { given Rich = new Rich("fixed"); import Fixed.conv; true.shout }
@main def run(): Unit =
  println(lower())
  println(ascribed())
  println(listed())
  println(fixed())
