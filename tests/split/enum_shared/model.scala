// Enums defined in one package and used from others: plain values, a companion with a body (which
// runs before a value is read), a stateful enum whose values the companion creates, and a class
// case.
package model

enum Suit:
  case Hearts, Spades

enum Level:
  case Low, High

object Level:
  println("Level companion initialised")
  val default: Level = High

enum Planet(val mass: Double):
  case Earth extends Planet(5.97)
  case Mars extends Planet(0.64)
  def heavier(than: Planet): Boolean = mass > than.mass

enum Shape:
  case Circle(r: Int)
  case Square(side: Int)
  case Dot

case class Card(suit: Suit, level: Level)
