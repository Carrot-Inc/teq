// `a == null` for an `a: A`: allowed without strictEquality, as scalac has it, for a type
// parameter that is not known to be a value type.
class Box[A](val value: A):
  def isNull: Boolean = value == null
  def orElse(other: A): A = if value != null then value else other
object Main:
  def isNull[A](a: A): Boolean = a == null
  def notNull[A <: AnyRef](a: A): Boolean = a != null
  def orDefault[A >: Null](a: A, d: A): A = if a == null then d else a
  def main(args: Array[String]): Unit =
    println(isNull("x"))
    println(isNull(null))
    println(isNull(1))
    println(notNull("y"))
    println(orDefault(null, "d"))
    println(orDefault("v", "d"))
    println(new Box[String](null).isNull)
    println(new Box("s").orElse("t"))
    println(new Box[String](null).orElse("t"))
