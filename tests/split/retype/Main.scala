package retype

object Main:
  def banner: String = "retype"

  def main(args: Array[String]): Unit =
    println(banner + " " + args.length)
    println(Card("ace").text + " " + Card("ace").counted + " " + cardShape)
    println(Fold.label + " " + Fold.sum + " " + Fold.kept)
    println(Panel.title + " " + Panel.counted + " " + Panel.shaped)
    println(Sides.picked)
    println(crateLabel(1) + " " + Crate(List(1, 2)).has(2))
