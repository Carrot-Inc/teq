case class Pt(x: Int = 1, y: Int = 2)
object W:
  extension (s: String)
    def comp: Pt.type = Pt
    export comp.{apply as mk}
object W2:
  extension (s: String)
    def comp: Pt.type = Pt
    export comp.apply
object Main:
  def m(): Int =
    object L:
      extension (s: String = "x") def lo: Int = 1
    class LC:
      extension (s: String = "y") def lc: Int = 2
    extension (s: String = "z") def loc: Int = 4
    { import L.*; "q".lo } + LC().lc("w") + "a".loc
  def main(args: Array[String]): Unit =
    println(m())
    import W.*
    println("a".mk(3, 4))
