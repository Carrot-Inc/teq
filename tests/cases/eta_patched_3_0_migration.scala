// Under 3.0-migration (`Scala2to3.needsPatch`) `x _` of a value is typed as `() => x` without -rewrite
// (`Migrations.typedAsFunction`): the expression evaluated at each call of the function, none before. The migration
// warning is silenced.
// teq: --source 3.0-migration --wconf msg=Only.function.types:s
object P {
  var n = 0
  def side: Int = { n += 1; 10 * n }
  def main(args: Array[String]): Unit = {
    val x = 1
    val f = x _
    println(f())
    val g = side _
    println(n)
    println(g() + g())
    val h = (x + 1) _
    println(h())
  }
}
