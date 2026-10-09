// Adapted from scala3 tests/run/blame_eye_triple_eee-double.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App, Scala 2 syntax of if, Scala 2 syntax of while.
object Test {
  def main(args: Array[String]): Unit = ()
  import Double.NaN

  // NaN must not equal NaN no matter what optimizations are applied
  // All the following will seem redundant, but to an optimizer
  // they can appear different

  val x = NaN

  if NaN == NaN then
    println("if (NaN == NaN) is broken")
  else
    println("if (NaN == NaN) is good")

  if x == x then
    println("if (x == x) is broken")
  else
    println("if (x == x) is good")

  if x == NaN then
    println("if (x == NaN) is broken")
  else
    println("if (x == NaN) is good")

  if NaN != NaN then
    println("if (NaN != NaN) is good")
  else
    println("if (NaN != NaN) broken")

  if x != x then
    println("if (x != x) is good")
  else
    println("if (x != x) broken")

  if NaN != x then
    println("if (NaN != x) is good")
  else
    println("if (NaN != x) is broken")

  x match {
    case 0.0d => println("x matched 0!")
    case NaN => println("x matched NaN!")
    case _ => println("x matching was good")
  }

  NaN match {
    case 0.0d => println("NaN matched 0!")
    case NaN => println("NaN matched NaN!")
    case _ => println("NaN matching was good")
  }

  var z = 0.0d
  var i = 0
  while i < 10 do {
    if i % 2 == 0 then z = NaN
    else z = NaN
    i += 1
  }
  if z.isNaN && i == 10 then println("loop with NaN was goood")
  else println("loop with NaN was broken")
}
