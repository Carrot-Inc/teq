object NullInput {
  object E {
    def unapply(s: String): Option[Int] = { println("unapply"); Some(1) }
  }
  def main(args: Array[String]): Unit = {
    val s: String = null
    println(s match { case E(n) => n; case _ => 0 })
  }
}
