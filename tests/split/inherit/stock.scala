// A class of each package extends a class of the other, so that a module names a superclass that
// another module defines, in both directions and before either has loaded.
package stock

import shop.*

abstract class Item(val name: String):
  println(s"Item $name")
  def price: Int
  def describe: String = s"$name costs $price"

trait Discounted extends Item:
  val percent: Int = 10
  override def describe: String = "discounted: " + name

class Bundle(name: String, parts: Int) extends Offer(name, parts * 2):
  override def describe: String = "bundle of " + super.describe
