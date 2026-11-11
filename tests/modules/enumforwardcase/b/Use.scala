// A downstream of a/ that completes the enum `Conn` before its companion: `Null <: Conn` asks whether the class
// is nullable (`isNullableClass`, its base classes), which reads its `Child` annotations, whose arguments are the
// companion's value cases by their definitions' addresses. Its reader makes each case's symbol where the reference
// stands (`symbolAt`, TreeUnpickler.scala:315–320) of the same record as the companion's index makes it
// (`createSymbol`, 606–617): a value case is read as the static field scalac writes, not an accessor of the
// companion (`Conn$.Or()`), and its class is the case's. (A sum mirror of `Conn` over the products lists the class
// case twice, products_enum_sum_mirror_cases_twice's, so this case summons none.)
package efb

import efa.*

object Use:
  def nul: Conn = (null: Conn)
  def or = Conn.Or
  def name(c: Conn): String = c match
    case Conn.And => "and"
    case Conn.Or => "or"
    case Conn.Not(d) => "not " + name(d)

@main def main(): Unit =
  println(Use.nul)
  println(Use.or)
  println(Use.name(Conn.Not(Conn.Or)))
  println(Conn.fromOrdinal(1))
  println(Conn.And.ordinal)
