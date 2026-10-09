package app

// An inline method of one package expanded at three sites of one shape in two others: the
// function they call and the anonymous class they share stand in the inline method's module.
@main def run(): Unit =
  println(p.width(3))
  println(p.height(-4))
  println(q.depth(30))
  println(lib.Fields.calls)
