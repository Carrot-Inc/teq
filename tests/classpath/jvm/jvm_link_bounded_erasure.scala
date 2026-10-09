// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// Results typed by a bounded higher-kinded parameter (`CC[K, V] <: MapOps[...]`, `C <: SetOps`)
// erase to the bound's class under scalac and to Object under teq; `Nothing` erases to
// scala.runtime.Nothing$; default getters take the earlier clauses' parameters only.
@main def run(): Unit =
  println(Set(1) + 2)
  println(Map(1 -> 2).updated(3, 4) - 1)
  println(Iterator(1, 2, 3, 4).sliding(2).toList)
  try assert(false, "boom") catch case e: AssertionError => println(e.getMessage)
