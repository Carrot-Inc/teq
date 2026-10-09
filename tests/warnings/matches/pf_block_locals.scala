// Whether a block that ends in a match gets E211 turns on what scalac's typer leaves of it: a
// match in braces of its own is a block, no match (the warning goes, the match is checked); a
// value whose type names a definition of the block, a class or a val, var or parameterless
// def, makes scalac ascribe the block's result (`ensureNoLocalRefs`), no match either; an
// application's type, an ascription's and a field's of a value are not the definition's.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Main:
  val nested: PartialFunction[Shape, Any] = s => { val k = 1; { s match { case Circle(r) => r + k } } }
  val commented: PartialFunction[Shape, Any] = s => { val k = 1; /* note */ { s match { case Circle(r) => r + k } } }
  val quoted: PartialFunction[Shape, Any] = s => { val u = "a//b"; { s match { case Circle(r) => r } } }
  val parens: PartialFunction[Shape, Any] = s => { val k = 1; (s match { case Circle(r) => r + k }) }
  val built: PartialFunction[Shape, Any] = s => { class C; s match { case Circle(r) => new C } }
  val listed: PartialFunction[Shape, Any] = s => { class C; s match { case Circle(r) => List(new C) } }
  val paired: PartialFunction[Shape, Any] = s => { class C; s match { case Circle(r) => (new C, r) } }
  val thunk: PartialFunction[Shape, Any] = s => { class C; s match { case Circle(r) => () => new C } }
  val made: PartialFunction[Shape, Any] = s => { class C; def mk(): C = new C; s match { case Circle(r) => mk() } }
  val caseClass: PartialFunction[Shape, Any] = s => { case class P(i: Int); s match { case Circle(r) => P(r) } }
  val widened: PartialFunction[Shape, Any] = s => { class C; s match { case Circle(r) => (new C: Any) } }
  val unused: PartialFunction[Shape, Any] = s => { class C; s match { case Circle(r) => r } }
  val applied: PartialFunction[Shape, Any] = s => { def k(): Int = 1; s match { case Circle(r) => k() } }
  val ascribed: PartialFunction[Shape, Any] = s => { val k = 1; s match { case Circle(r) => (k: Int) } }
  val inList: PartialFunction[Shape, Any] = s => { val k = 1; s match { case Circle(r) => List(k) } }
  val selected: PartialFunction[Shape, Any] = s => { val k = "x"; s match { case Circle(r) => k.length } }
  def main(args: Array[String]): Unit =
    val all = List(nested, commented, quoted, parens, built, listed, paired, thunk, made, caseClass, widened, unused, applied, ascribed, inList, selected)
    println(all.map(_.isDefinedAt(Square(0))))
