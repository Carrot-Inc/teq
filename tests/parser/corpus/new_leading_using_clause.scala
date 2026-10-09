class Ctx(val level: Int)

class Node(using val ctx: Ctx)(label: String, weight: Int):
  def show = s"${ctx.level} $label $weight"

class Pair()(using ctx: Ctx):
  def show = s"pair ${ctx.level}"

object Node:
  def make(using c: Ctx)(label: String): Node = new Node(using c)(label, label.length)

@main def main(): Unit =
  println(new Node(using Ctx(1))("a", 1).show)
  given Ctx = Ctx(2)
  println(new Node("b", 2).show)
  println(Node.make("ccc").show)
  println(new Pair()(using Ctx(3)).show)
  println(new Pair().show)
