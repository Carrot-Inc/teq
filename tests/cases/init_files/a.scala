package p
object Log:
  def t[A](label: String, a: A): A =
    println("  init " + label)
    a
import Log.t

val fileA1 = t("fileA1", 1)
val fileA2 = t("fileA2", 2)
lazy val fileALazy = t("fileALazy", 3)
var fileAVar = t("fileAVar", 4)
val fileAConst = 5
given fileAGiven: Ordering[Int] = t("fileAGiven", Ordering.Int.reverse)
val fileA3 = t("fileA3", fileA1 + fileA2)
