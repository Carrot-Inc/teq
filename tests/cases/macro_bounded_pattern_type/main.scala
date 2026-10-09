package app
import blib.Shape

case class Zero()
case class One(a: Int)
case class Two(a: Int, b: String)

@main def run(): Unit =
  println(Shape.of[Zero])
  println(Shape.of[One])
  println(Shape.of[Two])
