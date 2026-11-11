// jars: nesting-lib
// A Java class's member classes are entered at every depth (`enterOwnInnerClasses`, each child's own class file
// entering its children), so a path ten classes deep names the innermost.
import nest.Top

object Main:
  def main(args: Array[String]): Unit =
    val x: Top.N1.N2.N3.N4.N5.N6.N7.N8.N9.N10 = null
    println(x == null)
