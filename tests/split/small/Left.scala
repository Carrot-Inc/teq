// Under `--module-per-file ui` the two files of package ui are two modules that refer to each other in both
// directions, also in what runs when a module loads: a class of each registers with a trait of
// the other, an enum of each extends a trait of the other, and the objects and top-level vals of
// each read the other's enum values.
package ui

trait Shape:
  def name: String
  def describe: String = s"shape $name"

enum Tone extends Sized:
  case Loud, Quiet
  def size: Int = ordinal + 1

class Box(val w: Int) extends Sized:
  def size: Int = w * Flavor.Sweet.size + RightConst.base

object LeftConst:
  val text: String = "L:" + Flavor.Sour.describe

val leftTop: String = "leftTop " + Flavor.Sweet.size
