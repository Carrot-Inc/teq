package extractorclash

// A case class's synthetic `unapply` beside the companion's own, declared or inherited:
// withdrawn only where one matches its signature (`Namer.invalidateIfClashingSynthetic`), and
// present in the products wherever it stays (scalac downstream calls it).
case class C[A](x: A)
object C:
  def unapply(c: C[String]): Option[Int] = Some(c.x.length)

case class Box[A](x: A)
trait Extract:
  def unapply(b: Box[String]): Option[Int] = Some(99)
object Box extends Extract

case class Inh(x: Int)
trait Pick:
  def unapply(c: Inh): Option[Int] = Some(c.x + 10)
object Inh extends Pick
