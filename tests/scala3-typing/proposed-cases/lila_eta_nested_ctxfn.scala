// lila: Study.MyStudyPager(env.study.pager.mine, views.study.list.mineLikes)
// A method reference eta-expanded against (A, B) => C1 ?=> C2 ?=> R: scalac types the
// body `m(a, b)` against the expected context-function result, adding one context closure
// per layer (Typer.typedUnadapted -> makeContextualFunction, applied again to the body).
class Me(val n: String)
class Fmt(val s: String)
class Ctx(val c: String)

def mine(order: Int, page: Int)(using me: Me)(using f: Fmt): String = s"mine $order $page ${me.n} ${f.s}"
def mineLikes(order: Int, page: Int)(using c: Ctx): String = s"likes $order $page ${c.c}"

def pager(make: (Int, Int) => Me ?=> Fmt ?=> String): String =
  make(1, 2)(using Me("me"))(using Fmt("card"))
def render(view: (Int, Int) => Ctx ?=> Me ?=> String): String =
  view(3, 4)(using Ctx("ctx"))(using Me("me"))

@main def run() =
  println(pager(mine))
  println(render(mineLikes))
