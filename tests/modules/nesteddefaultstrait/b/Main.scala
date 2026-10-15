package ndt

object Main {
  def main(args: Array[String]): Unit = {
    val t: Trait = new Impl
    println(t.Item())
    println(t.Made()().z)
    println(t.Made(1)().z)
  }
}
