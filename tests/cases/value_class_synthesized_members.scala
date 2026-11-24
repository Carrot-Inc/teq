// A value class's synthesized members as scalac writes them: `equals` over each kind of underlying value (a
// primitive, a string, a class, an array, a reference that may hold a box, a type parameter), with a class's own
// `canEqual` and one a universal trait gives it, and a case value class's `productElement` and
// `productElementName`, an index out of range included. The cells are generated (the row-58 table of the value-class
// work, its portable rows).
case class K(i: Int)
class V_int(val u: Int) extends AnyVal
case class C_int(u: Int) extends AnyVal
class V_long(val u: Long) extends AnyVal
case class C_long(u: Long) extends AnyVal
class V_double(val u: Double) extends AnyVal
case class C_double(u: Double) extends AnyVal
class V_float(val u: Float) extends AnyVal
case class C_float(u: Float) extends AnyVal
class V_boolean(val u: Boolean) extends AnyVal
case class C_boolean(u: Boolean) extends AnyVal
class V_char(val u: Char) extends AnyVal
case class C_char(u: Char) extends AnyVal
class V_short(val u: Short) extends AnyVal
case class C_short(u: Short) extends AnyVal
class V_byte(val u: Byte) extends AnyVal
case class C_byte(u: Byte) extends AnyVal
class V_string(val u: String) extends AnyVal
case class C_string(u: String) extends AnyVal
class V_any(val u: Any) extends AnyVal
case class C_any(u: Any) extends AnyVal
class V_class(val u: K) extends AnyVal
case class C_class(u: K) extends AnyVal
class V_generic[A](val u: A) extends AnyVal
case class C_generic[A](u: A) extends AnyVal
class V_array(val u: Array[Int]) extends AnyVal
case class C_array(u: Array[Int]) extends AnyVal
class V_list(val u: List[Int]) extends AnyVal
case class C_list(u: List[Int]) extends AnyVal
case class Q_int(u: Int) extends AnyVal:
  override def canEqual(that: Any): Boolean = that.isInstanceOf[Q_int] && S.note("canEqual")
case class Q_string(u: String) extends AnyVal:
  override def canEqual(that: Any): Boolean = that.isInstanceOf[Q_string] && S.note("canEqual")
trait QE extends Any with Equals:
  override def canEqual(that: Any): Boolean = S.note("canEqual") && that.isInstanceOf[Q_inherited]
case class Q_inherited(u: Int) extends AnyVal with QE
object S:
  def note(s: String): Boolean = { print(s + " "); true }
  def str(x: Any): String = x match { case a: Array[?] => a.mkString("Array(", ",", ")") case null => "null" case _ => x.toString }
  def fails(x: => Any): String = try { x; "no" } catch { case e: Throwable => e.getClass.getName + ":" + e.getMessage }
  def out(name: String, xs: Any*): Unit = println(name + " " + xs.mkString(" "))
object m:
  def main(args: Array[String]): Unit =
    S.out("V_int eq", new V_int(1).equals(new V_int(1)), new V_int(1).equals(new V_int(2)), new V_int(1).equals("x"), new V_int(1).equals(null), new V_int(1) == new V_int(2))
    S.out("C_int eq", C_int(1).equals(C_int(1)), C_int(1).equals(C_int(2)), C_int(1).equals("x"), C_int(1).equals(null), C_int(1) == C_int(2))
    S.out("C_int pe", S.str(C_int(1).productElement(0)), C_int(1).productElementName(0), S.fails(C_int(1).productElement(1)), S.fails(C_int(1).productElementName(-1)))
    S.out("V_long eq", new V_long(1L).equals(new V_long(1L)), new V_long(1L).equals(new V_long(2L)), new V_long(1L).equals("x"), new V_long(1L).equals(null), new V_long(1L) == new V_long(2L))
    S.out("C_long eq", C_long(1L).equals(C_long(1L)), C_long(1L).equals(C_long(2L)), C_long(1L).equals("x"), C_long(1L).equals(null), C_long(1L) == C_long(2L))
    S.out("C_long pe", S.str(C_long(1L).productElement(0)), C_long(1L).productElementName(0), S.fails(C_long(1L).productElement(1)), S.fails(C_long(1L).productElementName(-1)))
    S.out("V_double eq", new V_double(1.5).equals(new V_double(1.5)), new V_double(1.5).equals(new V_double(-0.0)), new V_double(1.5).equals("x"), new V_double(1.5).equals(null), new V_double(1.5) == new V_double(-0.0))
    S.out("C_double eq", C_double(1.5).equals(C_double(1.5)), C_double(1.5).equals(C_double(-0.0)), C_double(1.5).equals("x"), C_double(1.5).equals(null), C_double(1.5) == C_double(-0.0))
    S.out("C_double pe", S.str(C_double(1.5).productElement(0)), C_double(1.5).productElementName(0), S.fails(C_double(1.5).productElement(1)), S.fails(C_double(1.5).productElementName(-1)))
    S.out("V_float eq", new V_float(1.5f).equals(new V_float(1.5f)), new V_float(1.5f).equals(new V_float(2.5f)), new V_float(1.5f).equals("x"), new V_float(1.5f).equals(null), new V_float(1.5f) == new V_float(2.5f))
    S.out("C_float eq", C_float(1.5f).equals(C_float(1.5f)), C_float(1.5f).equals(C_float(2.5f)), C_float(1.5f).equals("x"), C_float(1.5f).equals(null), C_float(1.5f) == C_float(2.5f))
    S.out("C_float pe", S.str(C_float(1.5f).productElement(0)), C_float(1.5f).productElementName(0), S.fails(C_float(1.5f).productElement(1)), S.fails(C_float(1.5f).productElementName(-1)))
    S.out("V_boolean eq", new V_boolean(true).equals(new V_boolean(true)), new V_boolean(true).equals(new V_boolean(false)), new V_boolean(true).equals("x"), new V_boolean(true).equals(null), new V_boolean(true) == new V_boolean(false))
    S.out("C_boolean eq", C_boolean(true).equals(C_boolean(true)), C_boolean(true).equals(C_boolean(false)), C_boolean(true).equals("x"), C_boolean(true).equals(null), C_boolean(true) == C_boolean(false))
    S.out("C_boolean pe", S.str(C_boolean(true).productElement(0)), C_boolean(true).productElementName(0), S.fails(C_boolean(true).productElement(1)), S.fails(C_boolean(true).productElementName(-1)))
    S.out("V_char eq", new V_char('a').equals(new V_char('a')), new V_char('a').equals(new V_char('b')), new V_char('a').equals("x"), new V_char('a').equals(null), new V_char('a') == new V_char('b'))
    S.out("C_char eq", C_char('a').equals(C_char('a')), C_char('a').equals(C_char('b')), C_char('a').equals("x"), C_char('a').equals(null), C_char('a') == C_char('b'))
    S.out("C_char pe", S.str(C_char('a').productElement(0)), C_char('a').productElementName(0), S.fails(C_char('a').productElement(1)), S.fails(C_char('a').productElementName(-1)))
    S.out("V_short eq", new V_short(1.toShort).equals(new V_short(1.toShort)), new V_short(1.toShort).equals(new V_short(2.toShort)), new V_short(1.toShort).equals("x"), new V_short(1.toShort).equals(null), new V_short(1.toShort) == new V_short(2.toShort))
    S.out("C_short eq", C_short(1.toShort).equals(C_short(1.toShort)), C_short(1.toShort).equals(C_short(2.toShort)), C_short(1.toShort).equals("x"), C_short(1.toShort).equals(null), C_short(1.toShort) == C_short(2.toShort))
    S.out("C_short pe", S.str(C_short(1.toShort).productElement(0)), C_short(1.toShort).productElementName(0), S.fails(C_short(1.toShort).productElement(1)), S.fails(C_short(1.toShort).productElementName(-1)))
    S.out("V_byte eq", new V_byte(1.toByte).equals(new V_byte(1.toByte)), new V_byte(1.toByte).equals(new V_byte(2.toByte)), new V_byte(1.toByte).equals("x"), new V_byte(1.toByte).equals(null), new V_byte(1.toByte) == new V_byte(2.toByte))
    S.out("C_byte eq", C_byte(1.toByte).equals(C_byte(1.toByte)), C_byte(1.toByte).equals(C_byte(2.toByte)), C_byte(1.toByte).equals("x"), C_byte(1.toByte).equals(null), C_byte(1.toByte) == C_byte(2.toByte))
    S.out("C_byte pe", S.str(C_byte(1.toByte).productElement(0)), C_byte(1.toByte).productElementName(0), S.fails(C_byte(1.toByte).productElement(1)), S.fails(C_byte(1.toByte).productElementName(-1)))
    S.out("V_string eq", new V_string("a").equals(new V_string("a")), new V_string("a").equals(new V_string(null)), new V_string("a").equals("x"), new V_string("a").equals(null), new V_string("a") == new V_string(null))
    S.out("C_string eq", C_string("a").equals(C_string("a")), C_string("a").equals(C_string(null)), C_string("a").equals("x"), C_string("a").equals(null), C_string("a") == C_string(null))
    S.out("C_string pe", S.str(C_string("a").productElement(0)), C_string("a").productElementName(0), S.fails(C_string("a").productElement(1)), S.fails(C_string("a").productElementName(-1)))
    S.out("V_any eq", new V_any(1).equals(new V_any(1)), new V_any(1).equals(new V_any("1")), new V_any(1).equals("x"), new V_any(1).equals(null), new V_any(1) == new V_any("1"))
    S.out("C_any eq", C_any(1).equals(C_any(1)), C_any(1).equals(C_any("1")), C_any(1).equals("x"), C_any(1).equals(null), C_any(1) == C_any("1"))
    S.out("C_any pe", S.str(C_any(1).productElement(0)), C_any(1).productElementName(0), S.fails(C_any(1).productElement(1)), S.fails(C_any(1).productElementName(-1)))
    S.out("V_class eq", new V_class(K(1)).equals(new V_class(K(1))), new V_class(K(1)).equals(new V_class(K(2))), new V_class(K(1)).equals("x"), new V_class(K(1)).equals(null), new V_class(K(1)) == new V_class(K(2)))
    S.out("C_class eq", C_class(K(1)).equals(C_class(K(1))), C_class(K(1)).equals(C_class(K(2))), C_class(K(1)).equals("x"), C_class(K(1)).equals(null), C_class(K(1)) == C_class(K(2)))
    S.out("C_class pe", S.str(C_class(K(1)).productElement(0)), C_class(K(1)).productElementName(0), S.fails(C_class(K(1)).productElement(1)), S.fails(C_class(K(1)).productElementName(-1)))
    S.out("V_generic eq", new V_generic[String]("g").equals(new V_generic[String]("g")), new V_generic[String]("g").equals(new V_generic[String](null)), new V_generic[String]("g").equals("x"), new V_generic[String]("g").equals(null), new V_generic[String]("g") == new V_generic[String](null))
    S.out("C_generic eq", C_generic[String]("g").equals(C_generic[String]("g")), C_generic[String]("g").equals(C_generic[String](null)), C_generic[String]("g").equals("x"), C_generic[String]("g").equals(null), C_generic[String]("g") == C_generic[String](null))
    S.out("C_generic pe", S.str(C_generic[String]("g").productElement(0)), C_generic[String]("g").productElementName(0), S.fails(C_generic[String]("g").productElement(1)), S.fails(C_generic[String]("g").productElementName(-1)))
    S.out("V_array eq", new V_array(Array(1)).equals(new V_array(Array(1))), new V_array(Array(1)).equals(new V_array(Array(1))), new V_array(Array(1)).equals("x"), new V_array(Array(1)).equals(null), new V_array(Array(1)) == new V_array(Array(1)))
    S.out("C_array eq", C_array(Array(1)).equals(C_array(Array(1))), C_array(Array(1)).equals(C_array(Array(1))), C_array(Array(1)).equals("x"), C_array(Array(1)).equals(null), C_array(Array(1)) == C_array(Array(1)))
    S.out("C_array pe", S.str(C_array(Array(1)).productElement(0)), C_array(Array(1)).productElementName(0), S.fails(C_array(Array(1)).productElement(1)), S.fails(C_array(Array(1)).productElementName(-1)))
    S.out("V_list eq", new V_list(List(1)).equals(new V_list(List(1))), new V_list(List(1)).equals(new V_list(Nil)), new V_list(List(1)).equals("x"), new V_list(List(1)).equals(null), new V_list(List(1)) == new V_list(Nil))
    S.out("C_list eq", C_list(List(1)).equals(C_list(List(1))), C_list(List(1)).equals(C_list(Nil)), C_list(List(1)).equals("x"), C_list(List(1)).equals(null), C_list(List(1)) == C_list(Nil))
    S.out("C_list pe", S.str(C_list(List(1)).productElement(0)), C_list(List(1)).productElementName(0), S.fails(C_list(List(1)).productElement(1)), S.fails(C_list(List(1)).productElementName(-1)))
    S.out("Q_int eq", Q_int(1).equals(Q_int(1)), Q_int(1).equals(Q_int(2)), Q_int(1).equals("x"))
    S.out("Q_string eq", Q_string("a").equals(Q_string("a")), Q_string("a").equals(Q_string("b")), Q_string("a").equals("x"))
    S.out("Q_inherited eq", Q_inherited(1).equals(Q_inherited(1)), Q_inherited(1).equals(Q_inherited(2)), Q_inherited(1).equals("x"))
