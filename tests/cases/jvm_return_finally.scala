// A return inside try/finally runs the enclosing finalizers on the JVM too (dotty: the finalizer on every exit); the shapes: a value, a finalizer returning, Unit, nested, in a catch case, in a loop, under operands, a throw
//> using scala 3.8.4
object Main:
  var log = List.empty[String]
  def note(s: String): Unit = log = s :: log
  def fin(): Int = try { return 1 } finally { note("fin") }
  def fin2(): Int = try { return 1 } finally { return 2 }
  def unit(): Unit = try { note("body"); return } finally { note("unit") }
  def nested(): String =
    try
      try { return "inner" } finally { note("n1") }
    finally note("n2")
  def inCatch(x: Int): Int =
    try { if x > 0 then throw new RuntimeException("x") else 0 }
    catch { case _: RuntimeException => return -1 }
    finally note("catch")
  def inLoop(): Int =
    var i = 0
    while i < 10 do
      try { if i == 3 then return i } finally { note("loop" + i) }
      i += 1
    -1
  def underOperands(c: Boolean): Int = 10 + (try { if c then return 5 else 1 } finally { note("ops") })
  def thrown(): Int = try { throw new RuntimeException("t") } finally { note("thrown") }
  def main(args: Array[String]): Unit =
    println(fin()); println(fin2()); unit(); println(nested()); println(inCatch(1)); println(inCatch(0))
    println(inLoop()); println(underOperands(true)); println(underOperands(false))
    println(try thrown() catch { case e: RuntimeException => "caught " + e.getMessage })
    println(log.reverse.mkString(","))
