package model

import lib.{Enumerated, HasLabel}

enum Color:
  case Red, Green

object Color:
  given Enumerated[Color] with
    def names: List[String] = List("red", "green")
    extension (e: Color) def entryName: String = names(e.ordinal)

sealed trait Animal:
  def sound: String
object Animal:
  given HasLabel[Animal] with
    extension (a: Animal) def label: String = "animal saying " + a.sound

final case class Dog(name: String) extends Animal:
  def sound: String = "woof"

object Shapes:
  final case class Square(side: Int)
  given HasLabel[Square] with
    extension (s: Square) def label: String = "square of " + s.side

final case class Secret(code: Int)
object Secret:
  private given hidden: HasLabel[Secret] with
    extension (s: Secret) def label: String = "secret " + s.code
  def reveal(s: Secret): String = s.label
