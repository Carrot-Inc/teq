// A part that is a member of a path no class bounds (`holder.T`, `H.T = A` through `Lib`'s `holder: H.type`) is the
// type the import's prefix gives, as a stuck match type is (`given_import_index_match_part`): scalac 3.8.4 prints 14.
trait Marker
class A extends Marker:
  def value = 14
trait Holder:
  type T
trait Library:
  val holder: Holder
  given mixed: (Marker & holder.T) = new A().asInstanceOf[Marker & holder.T]
object H extends Holder:
  type T = A
object Lib extends Library:
  val holder: H.type = H
object Main:
  def main(args: Array[String]): Unit =
    import Lib.mixed
    println(summon[A].value)
