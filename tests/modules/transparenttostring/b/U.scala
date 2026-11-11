package tcob

object Use:
  def a: String = tcoa.Fmt.show[Int]("alpha", 1, 10)
  def i: String = tcoa.Fmt.plain[Int](10)
  def z: String = tcoa.Fmt.plain[Boolean](true)
  def s: String = tcoa.Fmt.plain[String]("s")
