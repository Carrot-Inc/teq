// `Constant.convertTo` by range and target, whatever the tree (the review of `9e9b35b3d`, item 5): a singleton-typed
// call's `Int` constant within `Byte`'s, `Short`'s or `Char`'s range narrows; `adaptConstant` keeps an effectful tree,
// evaluated once (`n` is 2).
var n = 0
def id[T](x: T): T = x
def eff(): 65 = { n += 1; 65 }
val x: 65 = (id[65](65): Byte).toInt
val y: 65 = (eff(): Byte).toInt
val c: 65 = (eff(): Char).toInt
val s: 65 = (id[65](65): Short).toInt
@main def run(): Unit =
  println(s"$x $y $c $s $n")
