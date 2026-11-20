// A Java source's string, text block and character constants as the reference's scanner reads them (unicode escapes,
// then every backslash through the literal escapes), printed as UTF-16 code units, one line each.
object Main {
  def units(s: String): String = s.map(_.toInt).mkString(",")
  def main(args: Array[String]): Unit = {
    val ss = List(J.S01, J.S02, J.S03, J.S04, J.S05, J.S06, J.S07, J.S08, J.S09, J.S10, J.S11, J.T01)
    ss.foreach(s => println(units(s)))
    val cs = List(J.C01, J.C02, J.C03, J.C04, J.C05, J.C06)
    cs.foreach(c => println(c.toInt))
  }
}
