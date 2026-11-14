trait Reads2:
  def at(key: String): String
  def at(n: Int): Int

trait Writes2:
  def at(row: Int, col: Int, value: Int): Int
  def at(slot: Left.made.Slot): Int

class Rack2 { class Slot }

object Right:
  def make(x: Reads2 & Writes2) = { val r = new Rack2; List(1, 2, 3).map(_ + 1); x.at("s"); r }
  val made = make(null)
