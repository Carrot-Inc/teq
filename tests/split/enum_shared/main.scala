package app

import model.*
import view.*

@main def run(): Unit =
  println("start")
  println(show(Suit.Spades) + Suit.Spades.ordinal)
  println(allSuits)
  println(Deck.cards)
  println(Deck.heaviest)
  println(Planet.values.map(_.mass).toList)
  println(List(Shape.Circle(1), Shape.Dot, Shape.Square(2)).map(area))
  println(Level.valueOf("Low") == Level.Low)
  println(Card(Suit.Hearts, Level.Low) == Deck.cards.head)
