object Main {
  def till: List[String] = {
    class Till { type Coin = Int; class Drawer; type Bill = String }
    Lists.of[Till]
  }

  def safe: List[String] = {
    class Safe { class Door; type Key = Int }
    Lists.of[Safe]
  }

  def main(args: Array[String]): Unit = {
    till.foreach(println)
    safe.foreach(println)
  }
}
