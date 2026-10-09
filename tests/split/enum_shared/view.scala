package view

import model.*

def show(s: Suit): String = s match
  case Suit.Hearts => "♥"
  case Suit.Spades => "♠"

def area(s: Shape): Int = s match
  case Shape.Circle(r) => 3 * r * r
  case Shape.Square(side) => side * side
  case Shape.Dot => 0

val allSuits: List[String] = Suit.values.toList.map(show)

object Deck:
  val cards: List[Card] = List(Card(Suit.Hearts, Level.Low), Card(Suit.Spades, Level.default))
  def heaviest: Planet = if Planet.Earth.heavier(Planet.Mars) then Planet.Earth else Planet.Mars
