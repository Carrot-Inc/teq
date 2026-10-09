package ui
package app

import facade.{Badge, Gallery, IconSet, Token}

@main def run(): Unit =
  println(Gallery.all)
  println(IconSet.prefix + "x")
  val badge = Badge(1, "gold")
  badge.label = "silver"
  println(badge.label)
  println(Gallery.describe(badge))
  println(IconSet.reveal)
  println(IconSet.Nested.viaOuter)
  println(new Token("t1").raw)
