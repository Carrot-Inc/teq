package ibb

import iba.Bodies

@main def run(): Unit =
  println(Bodies.value(3))
  println(Bodies.code(1 + 2))
  val a = Bodies.make(3)
  val b = Bodies.make(4)
  println(s"${a()},${b()}")
  println(s"${Bodies.kindOf[Int]},${Bodies.kindOf[String]}")
  println(s"${Bodies.known(1)},${Bodies.known((1: Int))}")
  println(Bodies.value({ def y: Int = 3; 5 }))
