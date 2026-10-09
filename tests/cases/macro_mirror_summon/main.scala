package app
import mlib.*

enum Color derives Info:
  case Red, Green

case class Box(x: Int) derives Info

enum Late derives Info:
  case A
  case B(n: Int)

@main def run(): Unit =
  println(Info.probe[Color])
  println(Info.probe[Box])
  println(summon[Info[Color]].text)
  println(summon[Info[Box]].text)
  println(summon[Info[Late]].text)
