package localprefix

trait Domain:
  type Elem
  trait Parent:
    def choose(value: Elem): Int
    def choose(value: String): Int = value.length
  trait Other:
    def choose(value: Elem): Int
    def choose(value: String): Int
  abstract class Combined extends Parent with Other

trait Concrete extends Domain:
  type Elem = Int
  @scala.annotation.nowarn("msg=New anonymous class")
  inline def make(seed: Int): Combined =
    def nested(): Combined = new Combined:
      def choose(value: Int): Int = value + seed
    nested()

object Probe:
  import scala.quoted.*
  transparent inline def result(inline seed: Int): Int = ${ impl('seed) }
  def impl(seed: Expr[Int])(using Quotes): Expr[Int] =
    val factory = new Concrete {}
    val value = factory.make(seed.valueOrAbort)
    Expr(value.choose(41) + value.choose("outer"))
