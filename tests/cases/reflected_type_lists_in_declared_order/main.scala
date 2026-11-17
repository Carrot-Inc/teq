object Main {
  def till: List[String] = {
    class Till { type Coin = Int; class Drawer; type Bill = String }
    Lists.of[Till]
  }

  def main(args: Array[String]): Unit = {
    Lists.of[grove.Tree[Int, Int]].foreach(println)
    Lists.of[grove.Shrub].foreach(println)
    Lists.of[grove.Glade.type].foreach(println)
    Lists.of[grove.Noon].foreach(println)
    Lists.of[grove.Night].foreach(println)
    Lists.of[grove.Both].foreach(println)
    Lists.of[grove.Other].foreach(println)
    Lists.of[grove.First[Int, Int, Int]].foreach(println)
    Lists.of[grove.Spray].foreach(println)
    Lists.of[grove.Copse.type].foreach(println)
    Lists.of[grove.Hedge].foreach(println)
    Lists.of[grove.Thicket].foreach(println)
    Lists.of[grove.Board].foreach(println)
    Lists.of[grove.Deck].foreach(println)
    Lists.of[grove.Bin[Int]].foreach(println)
    Lists.of[grove.Hub].foreach(println)
    Lists.of[grove.Spoke].foreach(println)
    till.foreach(println)
  }
}
