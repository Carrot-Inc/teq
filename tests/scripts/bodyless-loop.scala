// A creation in a loop's later turns, which run as lowered code.
class Wide extends jlib.Span(3)

@main def run(): Unit =
  var i = 0
  var total = 0
  while i < 40 do
    if i == 30 then total += new Wide().n
    i += 1
  println(total)
