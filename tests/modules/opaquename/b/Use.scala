package onb

import ona.{Id, Name}

@main def run(): Unit =
  println(Name.of[Id])
  println(Name.of[Option[Id]])
