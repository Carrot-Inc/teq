package ca

case class Person(name: String, age: Int = 0)
case class Box[A](value: A, tags: List[String])
case class Pair(left: Int, right: Int):
  def sum: Int = left + right
