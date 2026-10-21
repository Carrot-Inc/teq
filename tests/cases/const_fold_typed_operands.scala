// dotty's `ConstFold` in the typer: an operand is a `ConstantTree` by its type, whatever the tree (a call, a block, a
// cast's target), an ascription by its own tree, `null` the null constant (`foldNullOp`), a type simplified (`"x" &
// String` is `"x"`); `classOf[T]` and a primitive's `getClass` are class constants (the review of `b23cfe2a3`, the table
// pass of the fourth fix-up). The trees keep their effects.
object O {
  def g: "x" = { println("g"); "x" }
  val calls: "xy" = g + "y"
  val block: "xy" = { println("block"); "x" } + "y"
  val right: "yx" = "y" + g
  val ascribedCall: "xy" = (g: String) + "y"
  val single: "ab" = ("a".asInstanceOf["a"]) + "b"
  val nullEq: false = "a" == null
  val nullNull: true = null == null
  val nullNe: true = "a" != null
  val glb: "xy" = ("x": "x" & String) + "y"
  val glbEq: true = (1: 1 & Int) == 1
  inline val intClass = classOf[Int]
  final val stringClass = classOf[String]
  final val boolClass = true.getClass
}

@main def run(): Unit =
  println(O.calls + " " + O.block + " " + O.right + " " + O.ascribedCall + " " + O.single)
  println(s"${O.nullEq} ${O.nullNull} ${O.nullNe} ${O.glb} ${O.glbEq}")
  println(O.intClass.getName + " " + O.stringClass.getName + " " + O.boolClass.getName)
