package app

import derivation.*
import widgets.Dropdown.dropdown

case class Point(x: Int) derives Eq

@main def run(): Unit =
  println(summon[Eq[Point]] != null)
  println(dropdown)
