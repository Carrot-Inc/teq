// lila: AppealTreeUi.topicMenu: Map[AppealTopic, Context ?=> Branch] = Map(AppealTopic.cheat -> engineMenu, ...)
// The argument of `->` is typed against a type variable whose upper bound is a context
// function type; scalac (Definitions.asContextFunctionType) looks through the bound and
// wraps `engineMenu` in a context closure instead of searching a given Context.
class Context(val n: Int)
def engineMenu(using c: Context): String = s"engine ${c.n}"
def boostMenu(using c: Context): String = s"boost ${c.n}"

val topicMenu: Map[Int, Context ?=> String] = Map(
  1 -> engineMenu,
  2 -> boostMenu
)

@main def run() =
  given Context = Context(3)
  println(topicMenu(1))
  println(topicMenu(2))
