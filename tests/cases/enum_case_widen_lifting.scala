// `widenEnumCase` (Applications.scala:1689-1704) widens a synthetic enum case `apply` only where the application is
// that `apply`'s tree: one that lifts a definition is `wrapDefs`' block, no symbol for the gate. `liftFun` lifts the
// written receiver where a default is used, the receiver is no literal, `this`, `new` or idempotent reference
// (`needLiftFun`, 589-605; `liftPrefix`, EtaExpansion.scala:164-170) and `LiftComplex.noLift` doesn't keep it as a
// pure path (186), and `liftArgs` the impure arguments of a reordered application. An export's forwarder of the
// companion is a stable reference, which stays in place (`refPurity`'s `StableRealizable`); ascribed, it is no
// reference and its object's initialisation makes it no pure path. A non-transparent inline call is expanded after
// the typer. The overload of `which` tells the case type from the widened enum.
enum E:
  case D(a: Int = 1, b: Int = 2)

object Ex:
  export E.D

object Main:
  def companion = E.D
  val stable = E.D
  inline def inl = E.D
  def which(x: E): String = "E"
  def which(x: E.D): String = "D"
  def main(args: Array[String]): Unit =
    println(which(companion.apply(b = 3)))
    println(which(companion.apply(1, 2)))
    println(which(stable.apply(b = 3)))
    println(which(E.D.apply(b = 3)))
    println(which(E.D(b = 3)))
    println(which(E.D(b = 3, a = 4)))
    println(which(E.D(b = { println("b"); 3 }, a = { println("a"); 4 })))
    println(which(companion(b = 3)))
    println(which(Ex.D()))
    println(which(Ex.D(b = 3)))
    println(which(Ex.D(b = { println("c"); 3 }, a = { println("d"); 4 })))
    println(which((E.D: E.D.type).apply(b = 3)))
    println(which((stable: E.D.type).apply(b = 3)))
    println(which((Ex.D: E.D.type).apply(b = 3)))
    println(which(inl.apply(b = 3)))
    println(which(({ E.D }).apply(b = 3)))
    println(which((E.D: E.D.type).apply()))
