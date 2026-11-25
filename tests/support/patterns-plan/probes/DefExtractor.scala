object DefExtractor {
  class Ex { def unapply(n: Int): Option[Int] = { println("unapply"); Some(n) } }
  def D: Ex = { println("D"); new Ex }
  def main(args: Array[String]): Unit = println(2 match {
    case D(1) => 1
    case D(2) => 2
    case _ => -1
  })
}
