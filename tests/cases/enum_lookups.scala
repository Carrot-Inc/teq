// The lookups of an enum's companion as dotty's `DesugarEnums.enumLookupMethods` makes them: an
// enum of values stores `$values` after its cases, which `values` clones and `fromOrdinal`
// reads, any failure of it a `NoSuchElementException`; `valueOf` reads the case of the name; an
// enum with a class case has `fromOrdinal` over its value cases alone. A case made while its
// companion is (a stateful enum's) sees what dotty's companion holds then.
enum E:
  case A, B
  val n: Int = try E.values.length catch case _: Throwable => -1
  val unmade: Boolean = Option(E.valueOf("B")).isEmpty
  val ord: String = try E.fromOrdinal(0).toString catch case e: NoSuchElementException => e.getMessage

enum M:
  case X
  case Y(k: Int)
  val unmade: Boolean = Option(M.fromOrdinal(0)).isEmpty

enum K:
  case P(k: Int)

object O:
  enum F:
    case G, H

def msg(f: => Any): String = try f.toString catch case e: Throwable => e.getMessage

@main def run(): Unit =
  println(List(E.A.n, E.B.n))
  println(List(E.A.unmade, E.B.unmade))
  println(List(E.A.ord, E.B.ord))
  println(M.X.unmade)
  println(E.values.toList)
  val a = E.values
  a(0) = E.B
  println(E.values.toList)
  println(s"${E.valueOf("A")} ${E.fromOrdinal(1)}")
  println(msg(E.valueOf("Z")))
  println(msg(E.fromOrdinal(5)))
  println(msg(E.fromOrdinal(-1)))
  println(s"${M.fromOrdinal(0)} ${msg(M.fromOrdinal(1))}")
  println(msg(K.fromOrdinal(0)))
  println(msg(O.F.valueOf("Z")) + " " + msg(O.F.fromOrdinal(2)) + " " + O.F.values.toList)
