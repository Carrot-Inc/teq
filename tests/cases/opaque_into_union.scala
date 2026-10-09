// An opaque type conforms to a union that holds its underlying type wherever its definition
// is visible; a bounded one conforms through its bound everywhere.
object Media:
  opaque type Raw = String
  object Raw:
    def apply(s: String): Raw = s
    def describe(r: Raw): String | Int | Long = r
  opaque type Count <: Int = Int
  object Count:
    def apply(n: Int): Count = n
  def widen(r: Raw): String | Int | Long | Double | Boolean = r
  def pick(r: Raw, flag: Boolean): String | Int = if flag then r else 1
  def all(rs: List[Raw]): List[String | Int] = rs
  extension (r: Raw) def value: String | Long = r

object Main:
  import Media.*
  def show(v: String | Int | Long | Double | Boolean): String = v match
    case s: String => s"str $s"
    case i: Int => s"int $i"
    case _ => "other"
  def main(args: Array[String]): Unit =
    val r = Raw("x")
    println(show(widen(r)))
    println(show(pick(r, true)))
    println(show(pick(r, false)))
    println(Raw.describe(r))
    println(all(List(Raw("a"), Raw("b"))))
    println(r.value)
    val c: Int | String = Count(3)
    println(show(c))
