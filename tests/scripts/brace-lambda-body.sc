// A script's statements with brace lambdas whose bodies begin on the line after the arrow: in an argument
// list, in a for's generator, under a `case`, and with a later line of the braces left of the body's first line,
// which the braces hold as a statement of the body.
val doubled = List(1, 2).map({ x =>
  val y = x * 2
  y
})
val pairs =
  for
    a <- List(1, 2).map { x =>
      val y = x + 1
      y
    }
    b <- List("p").collect { case s =>
      val t = s + a
      t
    }
  yield b
var log = ""
List(1, 2).foreach { x =>
    log += "a" + x
  log += "b" + x
}
List(3).foreach { case x =>
    log += "c" + x
  log += "d" + x
}
println(doubled)
println(pairs)
println(log)
