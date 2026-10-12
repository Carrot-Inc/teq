// A name of an overloaded method passed to an overloaded method is typed against the parameter of the
// alternative chosen, which picks the method's own alternative (dotty's FunProto.typedArgs, unadapted):
// lila's ByColor(fullIdOf) beside the case class's apply.
enum Color:
  case White, Black

case class ByColor[A](white: A, black: A)
object ByColor:
  def apply[A](f: Color => A): ByColor[A] = ByColor(f(Color.White), f(Color.Black))
  def make[A](f: Color => A): ByColor[A] = ByColor(f(Color.White), f(Color.Black))

class Game:
  def idOf(n: Int): Option[String] = None
  def idOf(color: Color): String = color.toString
  val f: Color => String = idOf
  def a: ByColor[String] = ByColor.make(idOf)
  def b: ByColor[String] = ByColor(idOf)

@main def run(): Unit =
  val g = Game()
  println(g.f(Color.White))
  println(g.a)
  println(g.b)
