// jars: scala-library
// std: scala-library
// A source object whose name ends in `$` beside one without, both nested in a class: the JVM
// names take the module suffix after the identifier as scalac does (`O$R$`, `O$R$$`), and the
// two are distinct.
class O:
  object R
  object `R$`

object Main:
  def main(args: Array[String]): Unit =
    val o = new O
    println(o.R.getClass.getName)
    println(o.`R$`.getClass.getName)
    println(o.R == o.`R$`)
