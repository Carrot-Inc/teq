// Packages a and b refer to each other in both directions, also in what runs when a module
// loads: a class of each registers with a trait of the other, an enum of each extends a trait of
// the other, and the objects and top-level vals of each read the other's enum values.
package a

import b.*

trait Named:
  def name: String
  def greet: String = s"hello from $name"

enum Color extends Named:
  case Red, Green
  def name: String = toString.toLowerCase

class Widget(val label: String) extends Tagged:
  def show: String = s"$label/$tag/${BConst.value}"

object AConst:
  val value: String = "A+" + Mode.Fast.name

val aTop: String = "aTop sees " + Mode.Slow.greet

def describe(c: Color): String = c match
  case Color.Red => "red: " + c.greet
  case Color.Green => "green: " + other(c)
