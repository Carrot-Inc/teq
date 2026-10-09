package sca

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
