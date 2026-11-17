// The second and third names are U+10000 and U+E000.
class Glyphs { type `𐀀` = Int; type `` = Int; type Plain = Int }

object Main {
  def main(args: Array[String]): Unit = {
    Lists.of[shelf.Stock].foreach(println)
    Lists.of[shelf.Outlet].foreach(println)
    Lists.of[shelf.Desk].foreach(println)
    Lists.of[shelf.Annex].foreach(println)
    println(Lists.units[Glyphs])
  }
}
