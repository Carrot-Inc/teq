package ibb

import iba.{Bodies, ByName, Ctors, Tag, Tests}

@main def run(): Unit =
  println(Bodies.value(3))
  println(Bodies.code(1 + 2))
  val a = Bodies.make(3)
  val b = Bodies.make(4)
  println(s"${a()},${b()}")
  println(s"${Bodies.kindOf[Int]},${Bodies.kindOf[String]}")
  println(s"${Bodies.known(1)},${Bodies.known((1: Int))}")
  println(Bodies.value({ def y: Int = 3; 5 }))
  println(Tag.derived(3))
  println(Tag.derived(300))
  println(Bodies.value({ val f: Int => Int = x => x + 1; 3 }))
  lazy val l: 3 = { println("init"); 3 }
  println(Bodies.both(l))
  println(Bodies.both(l))
  println(Ctors.make(7))
  println(Ctors.make2(4))
  val none: String = null
  println(s"${Tests.matches[String](none, true)},${Tests.matches[String]("s", true)},${Tests.matches[Int](3, true)}")
  println(s"${Tests.tester[String](true)(none)},${Tests.tester[String](true)("s")}")
  println(ByName.derive(3))
  println(ByName.derive2("s"))
