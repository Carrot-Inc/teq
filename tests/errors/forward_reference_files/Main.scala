// expect: Part01.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part02.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part03.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part04.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part05.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part06.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part07.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part08.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part09.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part10.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part11.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: Part12.scala:4:22: error: forward reference to second (defined on line 6) extends over the definition of held (on line 5)
// expect: 12 errors found
// A forward reference in each of twelve files: one error a file, the same at every worker count (tests/workers.sh).
object Test:
  def main(args: Array[String]): Unit = println(Part01.run(1) + Part02.run(1) + Part03.run(1) + Part04.run(1) + Part05.run(1) + Part06.run(1) + Part07.run(1) + Part08.run(1) + Part09.run(1) + Part10.run(1) + Part11.run(1) + Part12.run(1))
