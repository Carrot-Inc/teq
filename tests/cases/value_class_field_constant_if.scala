//> using platform jvm
// A value class's field selected on an `if` over a constant: dotty's FirstTransform leaves the branch taken alone
// before erasure, so the field is read by the box's accessor on that branch with no null test, and a null throws
// (the generic `missing[V]`). An `if` over a condition that is no constant adapts each branch to the value class, a
// null to its zero, and so does scalac.
class V(val n: Long) extends AnyVal
object Main:
  inline val On = true
  def missing[A]: A = null.asInstanceOf[A]
  def pick(c: Boolean): V = if c then missing[V] else new V(3L)
  def show(x: => Long): String = try x.toString catch { case _: NullPointerException => "null" }
  def main(args: Array[String]): Unit =
    println(show((if true then missing[V] else new V(3L)).n))
    println(show((if false then new V(3L) else missing[V]).n))
    println(show((if true then { println("block"); missing[V] } else new V(3L)).n))
    println(show((if On then missing[V] else new V(3L)).n))
    println(show((if !On then new V(4L) else missing[V]).n))
    println(show((if true then new V(5L) else missing[V]).n))
    println(show(missing[V].n))
    println(show(pick(true).n))
    println(show(pick(false).n))
