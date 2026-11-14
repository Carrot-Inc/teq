class Rack1 { class Slot }
class Rack2 { class Slot }
class Rack3 { class Slot }
class Rack4 { class Slot }

object Stock:
  val first = { val r = new Rack1; List(1, 2, 3).map(_ + 1); r }
  val second = { val r = new Rack2; List(1, 2, 3).map(_ + 1); r }
  val third = { val r = new Rack3; List(1, 2, 3).map(_ + 1); r }
  val fourth = { val r = new Rack4; List(1, 2, 3).map(_ + 1); r }

object ByKey { def read(x: Reads[Stock.first.Slot] & Writes[Stock.second.Slot]) = x.at("k") }
object ByCell { def read(x: Reads[Stock.fourth.Slot] & Writes[Stock.fourth.Slot]) = x.at(1, 2, 3) }
object Flipped { def read(x: Writes[Stock.second.Slot] & Reads[Stock.first.Slot]): String = x.at("t") }

class Both extends Reads[Stock.first.Slot] with Writes[Stock.second.Slot]:
  def at(key: String): String = key
  def at(slot: Stock.first.Slot): Int = 1
  def at(row: Int, col: Int, value: Int): Int = row + col + value
  def at(slot: Stock.third.Slot): Int = 3
  def at(slots: List[Stock.second.Slot]): Int = 4

class Fourths extends Reads[Stock.fourth.Slot] with Writes[Stock.fourth.Slot]:
  def at(key: String): String = key
  def at(slot: Stock.fourth.Slot): Int = 1
  def at(row: Int, col: Int, value: Int): Int = row + col + value
  def at(slot: Stock.third.Slot): Int = 3
  def at(slots: List[Stock.fourth.Slot]): Int = 4

@main def run(): Unit =
  println(ByKey.read(new Both) + ByCell.read(new Fourths) + Flipped.read(new Both))
