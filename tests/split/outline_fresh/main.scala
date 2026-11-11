package main

@main def run(): Unit =
  println(a.first(List(1, 2)))
  println(b.second(Nil))
  println(b.second(List(3)))
