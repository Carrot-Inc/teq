// expansions.scala with the type class's member a val its givens implement: each site reads it
// on the given object its search found, typed at that object's class, and the step's body is
// still written once, as the output reads the member alike on each.
import scala.compiletime.{constValue, summonInline}

trait Tag[A]:
  val name: String

object Tag:
  given Tag[Int] with
    val name = "int"
  given Tag[String] with
    val name = "string"
  given Tag[Boolean] with
    val name = "boolean"

trait Renderer:
  def renderShared(x: Int): String

object Steps:
  inline def step[L <: String, A](n: Int): String =
    val label = constValue[L]
    val tag = summonInline[Tag[A]]
    val prefix = if n > 1 then "many " else "one "
    prefix + label + ": " + tag.name + " / step body " + n.toString

  inline def renderer(k: Int): Renderer = new Renderer:
    def renderShared(x: Int): String = "rendered " + (x * k)

def more(): String =
  val k = 5
  Steps.renderer(k).renderShared(2)

@main def run(): Unit =
  val k = 3
  println(Steps.step["a", Int](1))
  println(Steps.step["b", String](2))
  println(Steps.step["c", Boolean](3))
  println(Steps.renderer(k).renderShared(1))
  println(Steps.renderer(k).renderShared(4))
  println(more())
