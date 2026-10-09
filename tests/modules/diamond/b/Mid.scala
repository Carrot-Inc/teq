package dib

import dia.*

class Cat(val name: String) extends Animal:
  override def sound: String = "meow"
object Zoo:
  def all: List[Animal] = List(Dog("rex"), new Cat("tom"))
