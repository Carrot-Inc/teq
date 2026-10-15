package ndt

object Client {
  def main(args: Array[String]): Unit = {
    val t: Trait = new Trait {}
    println(t.Item())
    println(t.Made()().z)
  }
}
