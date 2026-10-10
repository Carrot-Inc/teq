//> using platform jvm
// The program's `x.toString` is a call, which a `null` receiver throws on (dotty's
// `BCodeBodyBuilder.genApply` invokes it), whatever the receiver's type: a String's, an Object's,
// an Any's, a boxed number's, a class's own; a concatenation renders a `null` operand as `null`
// (`genStringConcat`), and a string by construction keeps its text. On JavaScript a member selected
// from `null` is the engine's `TypeError` (docs/COMPATIBILITY.md).
class C:
  override def toString = "C!"

object Main:
  def t(label: String)(body: => Any): Unit =
    try println(label + " " + body)
    catch case _: NullPointerException => println(label + " NPE")

  def main(args: Array[String]): Unit =
    val s: String = null
    val o: Object = null
    val a: Any = null
    val c: C = null
    val n: java.lang.Integer = null
    t("string")(s.toString)
    t("concat")("d " + s.toString)
    t("object")(o.toString)
    t("any")(a.toString)
    t("class")(c.toString)
    t("boxed")(n.toString)
    t("literal")("lit".toString)
    t("built")(("a" + s).toString)
    t("plain")("d " + s)
    t("interp")(s"d $s")
    var effects = 0
    def next(): String = { effects += 1; if effects > 1 then null else "x" }
    t("first")(next().toString)
    t("second")(next().toString)
    println(effects)
