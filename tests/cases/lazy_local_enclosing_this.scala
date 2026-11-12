// An anonymous class forcing a local lazy val whose initialiser reads a field of the enclosing class, beside a local
// def of the same body: dotty's LambdaLift lifts the accessor, the initialiser and the def into the enclosing class
// and calls them through its instance; teq writes the accessor and the initialiser there too, called with the
// instance the anonymous class holds beside its captures (`Gen::class_extras`), and copies the def into the
// anonymous class, that instance its `this`.
class Enclosing:
  val field = "enclosing field"
  def viaLazy(): String =
    lazy val l: String = field + " through a local lazy val"
    var seen = ""
    val r = new Runnable { def run(): Unit = seen = l }
    r.run()
    seen
  def viaDef(): String =
    def l(): String = field + " through a local def"
    var seen = ""
    val r = new Runnable { def run(): Unit = seen = l() }
    r.run()
    seen

object Main:
  def main(args: Array[String]): Unit =
    println(new Enclosing().viaLazy())
    println(new Enclosing().viaDef())
