package mo

// A product that overrides `productElement`, read by a case class's synthesized mirror at run time
// and in a macro's run: the override answers and runs once per element, its value and its count the
// same at both (scalac dispatches it); the case fields stand for the synthesized method alone.
import scala.deriving.Mirror
import scala.quoted.*
case class Out(x: Int)
object Count:
  var calls: Int = 0
case class Input(x: Int) extends Product:
  override def productElement(n: Int): Any =
    Count.calls += 1
    99
object Mac:
  inline def result: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    Count.calls = 0
    val out = summon[Mirror.ProductOf[Out]].fromProduct(Input(3))
    Expr(out.x.toString + ":" + Count.calls)
