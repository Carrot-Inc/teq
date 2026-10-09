package ova

class Printer:
  def print(x: Int): String = "int " + x
  def print(x: String): String = "str " + x
  def print(x: Int, y: Int): String = "two " + (x + y)

class Container[A](val items: List[A]):
  def first: A = items.head
  def mapped[B](f: A => B): Container[B] = new Container(items.map(f))
