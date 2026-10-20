// lila: appeal/ui/AppealTreeUi.scala:559 (`topic.flatMap(topicMenu.get) | (_ ?=> noTopicMenu(...))`)
// A context function literal `_ ?=> e` whose expected type is a type variable bounded by a
// context function type instantiates that variable to get its parameter type.
case class Ctx(n: Int)
sealed trait Node
case class Branch(s: String) extends Node

extension [A](self: Option[A]) inline def |(default: => A): A = self.getOrElse(default)

def menu(using c: Ctx): Branch = Branch(s"menu${c.n}")
def noTopic(using c: Ctx): Branch = Branch(s"none${c.n}")
val topicMenu: Map[String, Ctx ?=> Branch] = Map("a" -> ((c: Ctx) ?=> menu(using c)))
def render(n: Node): String = n.toString

def page(topic: Option[String])(using Ctx): String =
  render(topic.flatMap(topicMenu.get) | (_ ?=> noTopic))

@main def run() =
  given Ctx = Ctx(1)
  println(page(Some("a")))
  println(page(None))
