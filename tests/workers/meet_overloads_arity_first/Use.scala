// `at` selected on `Reads & Writes`: the member is the two parts' overloads met, each alternative of the
// second compared with the first's by their signatures (dotty's `Denotation.meet`), while other workers
// type the vals those signatures name.
class Rack1 { class Slot }
class Rack2 { class Slot }
class Rack3 { class Slot }
class Rack4 { class Slot }

object Stock:
  val first = { val r = new Rack1; List(1, 2, 3).map(_ + 1); r }
  val second = { val r = new Rack2; List(1, 2, 3).map(_ + 1); r }
  val third = { val r = new Rack3; List(1, 2, 3).map(_ + 1); r }
  val fourth = { val r = new Rack4; List(1, 2, 3).map(_ + 1); r }

object ByKey { def read(x: Reads & Writes) = x.at("k") }
object ByCell { def read(x: Reads & Writes) = x.at(1, 2, 3) }
object Flipped { def read(x: Writes & Reads): String = x.at("t") }

class Both extends Reads with Writes:
  def at(key: String): String = key
  def at(slot: Stock.first.Slot): Int = 1
  def at(row: Int, col: Int, value: Int): Int = row + col + value
  def at(slot: Stock.second.Slot): Int = 2
  def at(slot: Stock.third.Slot): Int = 3
  def at(slot: Stock.fourth.Slot): Int = 4

@main def run(): Unit =
  val both = new Both
  println(ByKey.read(both) + ByCell.read(both) + Flipped.read(both))
