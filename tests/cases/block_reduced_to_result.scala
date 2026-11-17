// teq: --wconf msg=A.pure.expression.does.nothing:s,msg=Discarded.non-Unit.value:s
// A block none of whose statements is written (a literal, a local, an inline method's parameter, `()`, an
// object without initialiser) is its result: the result is written as it would be standing in the block's
// place, whatever it is and wherever the block stands: a function literal next to an operator, a number as a
// receiver, a conditional as an operand or a condition.
class Box(val x: Int) { def get(): Int = x; override def toString: String = "Box(" + x + ")" }
object Quiet
inline def stmtThenValue(inline v: Any): Any = { v; v }
inline def first[A](inline label: String)(inline a: A): A = { label; a }
object Main {
  def show(a: Any): String = a match {
    case f: Function0[?] => "Function0 giving " + f()
    case f: Function1[?, ?] => "Function1 giving " + f.asInstanceOf[Int => Any](3)
    case b: Boolean => "Boolean " + b
    case s: String => "String " + s
    case u: Unit => "Unit"
    case o => o.getClass.getSimpleName.takeWhile(_ != '$') + " " + o
  }
  def inc(i: Int): Int = i + 1
  def id[A](a: A): A = a
  def main(args: Array[String]): Unit = {
    val local = 1
    val c = args.length == 0
    var v = 0
    val box = new Box(7)
    val arr = Array(10, 20, 30)
    // a function literal as the result
    println("01 != null: " + show({ 1; () => 1 } != null))
    println("02 == null: " + show({ local; (x: Int) => x } == null))
    println("03 null != it: " + show(null != { 1; () => 1 }))
    println("04 eq itself: " + show({ val f = { 1; () => 1 }; f eq f }))
    println("05 in a condition: " + (if ({ 1; () => true } != null) "yes" else "no"))
    println("06 applied: " + ({ 1; (x: Int) => x + 1 })(3))
    println("07 its apply selected: " + { 1; () => 5 }.apply())
    println("08 as an argument: " + show(id({ 1; () => 2 })))
    println("09 in a string: " + show("" + ({ 1; () => 1 } != null)))
    println("10 a tuple's element: " + show(({ 1; () => 3 }, 1)._1))
    println("11 a conditional's branch: " + show(if c then { 1; () => 4 } else { 1; () => 5 }))
    println("12 and: " + show(c && { 1; () => 1 } != null))
    println("13 an inline parameter, statement then value: " + show(stmtThenValue(() => 1) != null))
    println("14 an inline parameter after a label: " + show(first("inc")(inc) != null))
    println("15 a function's body: " + show((() => { 1; () => 6 })()))
    println("16 a method's reference: " + show({ 1; inc } != null))
    println("17 type test: " + show({ 1; () => 1 }.isInstanceOf[Function0[?]]))
    println("18 a val's value: " + show({ val f = { local; () => 7 }; f }))
    // a number as the result
    println("21 a receiver: " + { 1; 5 }.toString)
    println("22 a receiver, negative: " + { 1; -5 }.toString)
    println("23 a receiver, a Long: " + { 1; 5L }.toString)
    println("24 a receiver, a Double: " + { 1; 2.5 }.toString)
    println("25 minus it: " + (1 - { 1; -5 }))
    println("26 it minus: " + ({ 1; -5 } - 1))
    println("27 negated: " + -{ 1; 5 })
    println("28 times: " + ({ 1; 2 + 3 } * 2))
    println("29 an index: " + arr({ 1; 2 }))
    println("30 its hash: " + { 1; 5 }.hashCode)
    println("31 compared: " + ({ 1; 5 } > 4))
    println("32 max: " + { 1; 5 }.max(6))
    // a conditional as the result
    println("41 plus one: " + ({ 1; if c then 1 else 2 } + 1))
    println("42 as a condition: " + (if { 1; if c then true else false } then "yes" else "no"))
    println("43 a receiver: " + { 1; if c then "a" else "b" }.length)
    println("44 compared: " + ({ 1; if c then 1 else 2 } == 1))
    // other results
    println("51 a string's length: " + { 1; "abc" }.length)
    println("52 a creation's field: " + { 1; new Box(3) }.x)
    println("53 a creation's method: " + { 1; new Box(4) }.get())
    println("54 a local's field: " + { 1; box }.x)
    println("55 a block with an effect inside: " + ({ 1; { v += 1; v } } + 1))
    println("56 not: " + !{ 1; c })
    println("57 or: " + ({ 1; c } || false))
    println("58 an array's element: " + ({ 1; arr })(1))
    println("59 a string concatenation's length: " + { 1; "a" + local }.length)
    println("60 an assignment's value: " + show({ 1; v = 5 }))
    println("61 unit: " + show({ 1; () }))
    println("62 this: " + ({ 1; this } eq Main))
    println("63 an object: " + ({ 1; Quiet } eq Quiet))
    println("64 a tuple's first: " + { 1; (8, 9) }._1)
    println("65 a match: " + ({ 1; local match { case 1 => 10; case _ => 20 } } + 1))
    println("66 a comparison negated: " + !{ 1; local == 1 })
    println("67 a char: " + { 1; 'a' }.toInt)
    println("68 in an interpolation: " + s"${ { 1; 5 } }${ { 1; "s" } }")
    println("69 thrown and caught: " + (try { throw { 1; new RuntimeException("boom") } } catch { case e: RuntimeException => e.getMessage }))
    println("70 a scrutinee: " + ({ 1; local } match { case 1 => "one"; case _ => "other" }))
    // the statements master left out: () and an object without a body
    println("81 unit before a function: " + show({ (); () => 1 } != null))
    println("82 an object before a function: " + show({ Quiet; () => 1 } != null))
    println("83 unit before a number, a receiver: " + { (); 5 }.toString)
    // as statements
    { 1; () => 1 }
    { 1; 5 }
    { local; new Box(1) }
    println("end")
  }
}
