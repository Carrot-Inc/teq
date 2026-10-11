// Every enum's companion has `fromOrdinal`, a mixed one's too, whose class cases are holes that throw
// `NoSuchElementException` as an ordinal out of range does (dotty's `DesugarEnums.enumLookupMethods`; master
// rejected the member where a case takes parameters); `values` and `valueOf` stay an enumeration's alone.
enum E { case A; case B(x: Int); case C }
enum F { case P(x: Int); case Q(y: String) }
enum G[T] { case K extends G[Int]; case L(t: String) extends G[String] }
def show(f: => Any): String = try f.toString catch { case e: NoSuchElementException => e.getMessage }
@main def run() =
  println(show(E.fromOrdinal(0)))
  println(show(E.fromOrdinal(1)))
  println(show(E.fromOrdinal(2)))
  println(show(E.fromOrdinal(3)))
  println(show(E.fromOrdinal(-1)))
  println(show(F.fromOrdinal(0)))
  println(show(G.fromOrdinal(0)))
  println(show(G.fromOrdinal(1)))
