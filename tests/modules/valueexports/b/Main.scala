import lib.*

@main def main(): Unit =
  val a = Advice(Info(3, "white"), "t")
  println((a.ply, a.color, a.desc("p"), a.desc("p", 2)))
  val g = Game(Info(5, "black"))
  println((g.ply, g.describe("q"), g.twice('x')))
  import g.*
  println(color)
  val e: Ext = Ctrl(Net("https://x"))
  println(e.baseUrl)
