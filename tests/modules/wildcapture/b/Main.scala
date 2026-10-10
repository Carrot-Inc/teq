package wcb

import wca.*

case class Person(name: String) extends Named

@main def run(): Unit =
  println(Api.sizes(List(Box(7, "s"), Box(8, 1))))
  println(Api.names(List(Box(1, Person("ann")))))
  println(Api.firsts(List(Box(1, "x"), Box(2, 3))))
