package cdc

object Main {
  def main(args: Array[String]): Unit = {
    println(new Top()().z)
    println(new Top(4)().z)
    println(TopCase(3)().z)
    println(new Holder.In("q")().b)
  }
}
