// A cycle through an overload's signature, across two files: `Left.make`'s result is inferred from a
// body that selects `at` on `Reads1 & Writes1`, whose meet compares the overload that names
// `Right.made`, and Right.scala is this file's mirror. scalac refuses the program (E044); one worker
// reports the recursive use, and so does every schedule of several.
trait Reads1:
  def at(key: String): String
  def at(n: Int): Int

trait Writes1:
  def at(row: Int, col: Int, value: Int): Int
  def at(slot: Right.made.Slot): Int

class Rack1 { class Slot }

object Left:
  def make(x: Reads1 & Writes1) = { val r = new Rack1; List(1, 2, 3).map(_ + 1); x.at("s"); r }
  val made = make(null)
